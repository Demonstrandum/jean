//! A minimal VNC (RFB 3.8) server for a 1-bit framebuffer: no authentication, Raw encoding, 32-bit true colour.
//! Clients that ask for an incremental update get only the rectangle that changed since the frame they last had.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use crate::{damage_rects, Frame, Rect};

/// The frame being shown, and a version that moves whenever it changes.
pub struct Screen {
    state: Mutex<(u64, Frame)>,
    changed: Condvar,
}

impl Screen {
    pub fn new(first: Frame) -> Arc<Screen> {
        Arc::new(Screen { state: Mutex::new((1, first)), changed: Condvar::new() })
    }

    /// Show `frame`; clients are woken only if it differs from what is shown.
    pub fn show(&self, frame: Frame) {
        let mut s = self.state.lock().unwrap();
        if s.1 != frame {
            *s = (s.0 + 1, frame);
            self.changed.notify_all();
        }
    }

    /// How many rectangles, and how many pixels, differ between the frame on show and `next` (for diagnosing refresh traffic).
    pub fn damage_to(&self, next: &Frame) -> (usize, usize) {
        let st = self.state.lock().unwrap();
        let rects = damage_rects(&st.1, next);
        (rects.len(), rects.iter().map(|r| r.w * r.h).sum())
    }

    pub fn size(&self) -> (usize, usize) {
        let s = self.state.lock().unwrap();
        (s.1.w, s.1.h)
    }
}

fn read_exact(s: &mut TcpStream, n: usize) -> std::io::Result<Vec<u8>> {
    let mut b = vec![0; n];
    s.read_exact(&mut b)?;
    Ok(b)
}

/// A private encoding for 1-bit frames: the rectangle as bits, row by row, each row padded to a byte, most significant bit first, 1 = white. A viewer that
/// lists it in SetEncodings (the tablet's does) gets a 32nd of the bytes Raw would take; any other viewer gets Raw.
pub const ENC_PACKED: i32 = 0x6A65_0001;

/// Pixels of `r` as packed bits (see `ENC_PACKED`).
fn packed(frame: &Frame, r: Rect) -> Vec<u8> {
    let row_bytes = r.w.div_ceil(8);
    let mut out = vec![0u8; row_bytes * r.h];
    for y in 0..r.h {
        for x in 0..r.w {
            if frame.px[(r.y + y) * frame.w + r.x + x] >= 128 {
                out[y * row_bytes + x / 8] |= 0x80 >> (x % 8);
            }
        }
    }
    out
}

/// Pixels of `r` as the client's 32-bit little-endian BGRX (the pixel format this server announces).
fn raw(frame: &Frame, r: Rect) -> Vec<u8> {
    let mut out = Vec::with_capacity(r.w * r.h * 4);
    for y in r.y..r.y + r.h {
        for x in r.x..r.x + r.w {
            let v = frame.px[y * frame.w + x];
            out.extend_from_slice(&[v, v, v, 0]);
        }
    }
    out
}

/// Milliseconds since the first call: one clock for the debug lines of the server's stages.
pub fn clock_ms() -> u128 {
    static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    START.get_or_init(std::time::Instant::now).elapsed().as_millis()
}

/// What a finger did on the panel, in frame pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gesture {
    /// Down and up in the same place: where it began.
    Tap { x: usize, y: usize },
    /// Down, a long way across, up.
    Swipe { from: (usize, usize), to: (usize, usize) },
}

/// What to do with a gesture.
pub type OnGesture = Arc<dyn Fn(Gesture) + Send + Sync>;

/// Press and release within this many frame pixels of each other is a tap; at least `SWIPE_MIN` apart is a swipe; in between is neither (a wobble).
const TAP_SLOP: usize = 60;
const SWIPE_MIN: usize = 240;

/// One client. Two threads: this one answers framebuffer requests (waiting for the screen to change when it is asked for what changed), and a reader
/// that takes everything the client sends. They have to be apart: while this thread waits for a frame to change it reads nothing, so with one thread a
/// touch was not even seen until the next change of the screen, which made a tap's response anywhere from a moment to a second late.
fn serve_client(mut s: TcpStream, screen: Arc<Screen>, on_gesture: OnGesture) -> std::io::Result<()> {
    s.write_all(b"RFB 003.008\n")?;
    read_exact(&mut s, 12)?;
    s.write_all(&[1, 1])?; // one security type: None
    read_exact(&mut s, 1)?;
    s.write_all(&[0, 0, 0, 0])?; // SecurityResult OK
    read_exact(&mut s, 1)?; // ClientInit (shared flag)
    let (w, h) = screen.size();
    let mut init = Vec::new();
    init.extend_from_slice(&(w as u16).to_be_bytes());
    init.extend_from_slice(&(h as u16).to_be_bytes());
    // pixel format: 32 bpp, depth 24, little endian, true colour, 8 bits per channel, red shift 16, green 8, blue 0
    init.extend_from_slice(&[32, 24, 0, 1, 0, 255, 0, 255, 0, 255, 16, 8, 0, 0, 0, 0]);
    let name = b"jean";
    init.extend_from_slice(&(name.len() as u32).to_be_bytes());
    init.extend_from_slice(name);
    s.write_all(&init)?;

    let (requests, asked) = std::sync::mpsc::channel::<bool>();
    let wants_packed = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let mut reader = s.try_clone()?;
    let reader_packed = wants_packed.clone();
    std::thread::spawn(move || -> std::io::Result<()> {
        let mut pressed: Option<(usize, usize)> = None;
        loop {
            match read_exact(&mut reader, 1)?[0] {
                0 => {
                    read_exact(&mut reader, 19)?; // SetPixelFormat: the client's wishes are ignored, the announced format is always used
                }
                2 => {
                    let n = u16::from_be_bytes(read_exact(&mut reader, 3)?[1..3].try_into().unwrap()) as usize;
                    let list = read_exact(&mut reader, 4 * n)?;
                    reader_packed.store(list.chunks_exact(4).any(|e| i32::from_be_bytes(e.try_into().unwrap()) == ENC_PACKED), std::sync::atomic::Ordering::Relaxed);
                }
                3 => {
                    let req = read_exact(&mut reader, 9)?;
                    if requests.send(req[0] != 0).is_err() {
                        return Ok(());
                    }
                }
                4 => {
                    read_exact(&mut reader, 7)?; // key event
                }
                5 => {
                    let p = read_exact(&mut reader, 5)?; // pointer event: button mask, x, y
                    let (x, y) = (u16::from_be_bytes([p[1], p[2]]) as usize, u16::from_be_bytes([p[3], p[4]]) as usize);
                    match (p[0] & 1 != 0, pressed) {
                        (true, None) => pressed = Some((x, y)),
                        (false, Some((px, py))) => {
                            pressed = None;
                            let far = px.abs_diff(x).max(py.abs_diff(y));
                            if far <= TAP_SLOP {
                                on_gesture(Gesture::Tap { x: px, y: py });
                            } else if far >= SWIPE_MIN {
                                on_gesture(Gesture::Swipe { from: (px, py), to: (x, y) });
                            }
                        }
                        _ => {}
                    }
                }
                6 => {
                    let n = u32::from_be_bytes(read_exact(&mut reader, 7)?[3..7].try_into().unwrap()) as usize;
                    read_exact(&mut reader, n.min(1 << 20))?; // client cut text
                }
                other => return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, format!("unknown client message {other}"))),
            }
        }
    });

    let mut had: Option<(u64, Frame)> = None;
    while let Ok(incremental) = asked.recv() {
        // a full request is answered at once, an incremental one when the frame changes
        let mut st = screen.state.lock().unwrap();
        while incremental && had.as_ref().is_some_and(|(v, _)| *v == st.0) {
            st = screen.changed.wait_timeout(st, Duration::from_secs(30)).unwrap().0;
        }
        let (version, frame) = (st.0, st.1.clone());
        drop(st);
        let rects = match (&had, incremental) {
            (Some((_, old)), true) => damage_rects(old, &frame),
            _ => vec![Rect { x: 0, y: 0, w: frame.w, h: frame.h }],
        };
        let packed_ok = wants_packed.load(std::sync::atomic::Ordering::Relaxed) && frame.px.iter().all(|&p| p == 0 || p == 255);
        let mut msg = vec![0, 0];
        msg.extend_from_slice(&(rects.len() as u16).to_be_bytes());
        for r in rects {
            for v in [r.x, r.y, r.w, r.h] {
                msg.extend_from_slice(&(v as u16).to_be_bytes());
            }
            if packed_ok {
                msg.extend_from_slice(&ENC_PACKED.to_be_bytes());
                msg.extend_from_slice(&packed(&frame, r));
            } else {
                msg.extend_from_slice(&0i32.to_be_bytes()); // Raw
                msg.extend_from_slice(&raw(&frame, r));
            }
        }
        let (n_rects, bytes) = (u16::from_be_bytes([msg[2], msg[3]]), msg.len());
        let t_send = std::time::Instant::now();
        s.write_all(&msg)?;
        if n_rects > 40 && std::env::var_os("EINK_DEBUG").is_some() {
            eprintln!("eink-serve: sent {n_rects} rects, {bytes} bytes in {} ms (at {} ms)", t_send.elapsed().as_millis(), clock_ms());
        }
        had = Some((version, frame));
    }
    Ok(())
}

/// Listen on `addr` and serve `screen` to every client. Returns the address actually bound.
pub fn serve(addr: &str, screen: Arc<Screen>, on_gesture: OnGesture) -> std::io::Result<std::net::SocketAddr> {
    let listener = TcpListener::bind(addr)?;
    let bound = listener.local_addr()?;
    std::thread::spawn(move || {
        for conn in listener.incoming().flatten() {
            let (screen, on_gesture) = (screen.clone(), on_gesture.clone());
            std::thread::spawn(move || {
                let _ = serve_client(conn, screen, on_gesture);
            });
        }
    });
    Ok(bound)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Just enough of a viewer: handshake, then framebuffer requests.
    struct Viewer(TcpStream);
    impl Viewer {
        fn connect(addr: std::net::SocketAddr) -> (Viewer, (usize, usize)) {
            let mut s = TcpStream::connect(addr).unwrap();
            s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
            assert_eq!(read_exact(&mut s, 12).unwrap(), b"RFB 003.008\n");
            s.write_all(b"RFB 003.008\n").unwrap();
            assert_eq!(read_exact(&mut s, 2).unwrap(), [1, 1]);
            s.write_all(&[1]).unwrap();
            assert_eq!(read_exact(&mut s, 4).unwrap(), [0, 0, 0, 0]);
            s.write_all(&[1]).unwrap();
            let head = read_exact(&mut s, 24).unwrap();
            let size = (u16::from_be_bytes([head[0], head[1]]) as usize, u16::from_be_bytes([head[2], head[3]]) as usize);
            let n = u32::from_be_bytes(head[20..24].try_into().unwrap()) as usize;
            assert_eq!(read_exact(&mut s, n).unwrap(), b"jean");
            (Viewer(s), size)
        }

        /// Ask for an update; the rectangles received as (rect, pixels).
        fn request(&mut self, incremental: bool, w: usize, h: usize) -> Vec<(Rect, Vec<u8>)> {
            let mut m = vec![3, incremental as u8, 0, 0, 0, 0];
            m.extend_from_slice(&(w as u16).to_be_bytes());
            m.extend_from_slice(&(h as u16).to_be_bytes());
            self.0.write_all(&m).unwrap();
            let head = read_exact(&mut self.0, 4).unwrap();
            assert_eq!(head[0], 0);
            let count = u16::from_be_bytes([head[2], head[3]]) as usize;
            (0..count)
                .map(|_| {
                    let r = read_exact(&mut self.0, 12).unwrap();
                    let v = |i: usize| u16::from_be_bytes([r[i], r[i + 1]]) as usize;
                    let rect = Rect { x: v(0), y: v(2), w: v(4), h: v(6) };
                    assert_eq!(i32::from_be_bytes(r[8..12].try_into().unwrap()), 0, "Raw encoding");
                    (rect, read_exact(&mut self.0, rect.w * rect.h * 4).unwrap())
                })
                .collect()
        }
    }

    #[test]
    fn a_viewer_gets_the_whole_frame_and_then_only_what_changed() {
        let mut f = Frame::white(16, 8);
        f.px[0] = 0;
        let screen = Screen::new(f.clone());
        let addr = serve("127.0.0.1:0", screen.clone(), Arc::new(|_| {})).unwrap();
        let (mut v, size) = Viewer::connect(addr);
        assert_eq!(size, (16, 8));
        let full = v.request(false, 16, 8);
        assert_eq!(full[0].0, Rect { x: 0, y: 0, w: 16, h: 8 });
        assert_eq!(&full[0].1[0..4], &[0, 0, 0, 0], "the black pixel");
        assert_eq!(&full[0].1[4..8], &[255, 255, 255, 0], "a white pixel");
        // nothing changes until it does: the incremental request waits, then gets just the box
        let s2 = screen.clone();
        let t = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(150));
            let mut g = f.clone();
            g.px[3 * 16 + 5] = 0;
            g.px[4 * 16 + 9] = 0;
            s2.show(g);
        });
        let inc = v.request(true, 16, 8);
        t.join().unwrap();
        assert_eq!(inc.len(), 1);
        assert_eq!(inc[0].0, Rect { x: 5, y: 3, w: 5, h: 2 });
        assert_eq!(&inc[0].1[0..4], &[0, 0, 0, 0]);
    }

    #[test]
    fn showing_the_same_frame_does_not_wake_anyone() {
        let f = Frame::white(4, 4);
        let screen = Screen::new(f.clone());
        let before = screen.state.lock().unwrap().0;
        screen.show(f);
        assert_eq!(screen.state.lock().unwrap().0, before);
    }

    #[test]
    fn a_press_and_release_close_together_is_a_tap_and_a_long_stroke_a_swipe() {
        let seen: Arc<Mutex<Vec<Gesture>>> = Default::default();
        let s2 = seen.clone();
        let screen = Screen::new(Frame::white(16, 8));
        let addr = serve("127.0.0.1:0", screen, Arc::new(move |g| s2.lock().unwrap().push(g))).unwrap();
        let (mut v, _) = Viewer::connect(addr);
        let pointer = |v: &mut Viewer, mask: u8, x: u16, y: u16| {
            let mut m = vec![5, mask];
            m.extend_from_slice(&x.to_be_bytes());
            m.extend_from_slice(&y.to_be_bytes());
            v.0.write_all(&m).unwrap();
        };
        pointer(&mut v, 1, 700, 300);
        pointer(&mut v, 0, 705, 302); // a tap, where it began
        pointer(&mut v, 1, 100, 100);
        pointer(&mut v, 0, 200, 100); // a wobble: neither
        pointer(&mut v, 1, 1000, 500);
        pointer(&mut v, 0, 400, 520); // a swipe
        v.request(false, 16, 8); // the server has handled everything sent before this
        assert_eq!(*seen.lock().unwrap(), vec![Gesture::Tap { x: 700, y: 300 }, Gesture::Swipe { from: (1000, 500), to: (400, 520) }]);
    }

    #[test]
    fn a_viewer_that_asks_for_packed_bits_gets_a_32nd_of_the_bytes() {
        let mut f = Frame::white(16, 8);
        f.px[0] = 0; // top-left black
        f.px[9] = 0; // x=9, y=0 black
        let screen = Screen::new(f);
        let addr = serve("127.0.0.1:0", screen, Arc::new(|_| {})).unwrap();
        let (mut v, _) = Viewer::connect(addr);
        let mut m = vec![2, 0, 0, 1];
        m.extend_from_slice(&ENC_PACKED.to_be_bytes());
        v.0.write_all(&m).unwrap();
        let mut req = vec![3, 0, 0, 0, 0, 0];
        req.extend_from_slice(&16u16.to_be_bytes());
        req.extend_from_slice(&8u16.to_be_bytes());
        v.0.write_all(&req).unwrap();
        let head = read_exact(&mut v.0, 4).unwrap();
        assert_eq!(u16::from_be_bytes([head[2], head[3]]), 1);
        let r = read_exact(&mut v.0, 12).unwrap();
        assert_eq!(i32::from_be_bytes(r[8..12].try_into().unwrap()), ENC_PACKED);
        let bits = read_exact(&mut v.0, 2 * 8).unwrap(); // 16 px wide = 2 bytes a row, 8 rows
        assert_eq!(bits[0], 0b0111_1111, "the first pixel is black, the rest of the byte white");
        assert_eq!(bits[1], 0b1011_1111, "pixel 9 is black");
        assert_eq!(bits[2], 0xFF);
    }
}
