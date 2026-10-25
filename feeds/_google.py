"""Google service-account access (stdlib + the openssl CLI for the RS256 signature). Files starting with _ are not run as feeds."""
import base64, json, os, subprocess, tempfile, time, urllib.error, urllib.parse, urllib.request
from _lib import ROOT, secrets

_tok = {"v": None, "exp": 0}
b64 = lambda b: base64.urlsafe_b64encode(b).rstrip(b"=").decode()


def google_cfg():
    return secrets().get("google", {})


def token(scope="https://www.googleapis.com/auth/calendar"):
    if _tok["v"] and time.time() < _tok["exp"] - 60:
        return _tok["v"]
    g = google_cfg()
    sa = json.load(open(os.path.join(ROOT, g["service_account_file"])))
    now = int(time.time())
    head = b64(json.dumps({"alg": "RS256", "typ": "JWT"}).encode())
    claim = b64(json.dumps({"iss": sa["client_email"], "scope": scope, "aud": sa["token_uri"], "iat": now, "exp": now + 3000}).encode())
    with tempfile.NamedTemporaryFile("w", delete=False) as f:
        os.chmod(f.name, 0o600)
        f.write(sa["private_key"])
    try:
        sig = subprocess.run(["openssl", "dgst", "-sha256", "-sign", f.name], input=f"{head}.{claim}".encode(), capture_output=True, check=True).stdout
    finally:
        os.unlink(f.name)
    body = urllib.parse.urlencode({"grant_type": "urn:ietf:params:oauth:grant-type:jwt-bearer", "assertion": f"{head}.{claim}.{b64(sig)}"}).encode()
    r = json.load(urllib.request.urlopen(urllib.request.Request(sa["token_uri"], body), timeout=20))
    _tok["v"], _tok["exp"] = r["access_token"], time.time() + r.get("expires_in", 3000)
    return _tok["v"]


def calendar_api(method, path="", body=None, params=None):
    """Call the Calendar API on the configured calendar. path is relative to /calendars/<id>."""
    cid = urllib.parse.quote(google_cfg()["calendar_id"], safe="")
    url = f"https://www.googleapis.com/calendar/v3/calendars/{cid}{path}"
    if params:
        url += "?" + urllib.parse.urlencode(params)
    req = urllib.request.Request(url, method=method, data=json.dumps(body).encode() if body is not None else None,
                                 headers={"Authorization": "Bearer " + token(), "Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=20) as r:
            raw = r.read()
            return json.loads(raw) if raw else {}
    except urllib.error.HTTPError as e:
        raise RuntimeError(f"calendar {method} {path}: HTTP {e.code} {e.read()[:200].decode(errors='replace')}")
