#!/usr/bin/env python3
"""Make a Qt binary resource file (the `resources.rcc` an AppLoad app carries) from files, without Qt's `rcc`.

    mkrcc.py out.rcc /ui/main.qml=path/to/main.qml [/other/file=path ...]

The format (format version 2, which every Qt 5.x/6.x reads): a 20-byte header ("qres", version, and the offsets of the tree, the data and the names), a tree of
22-byte nodes (name offset, flags, then either child count and first child index for a directory, or territory, language (1, the C locale) and data offset for a file, then a
modification time), the file data (a length, then the bytes) and the names (a length in UTF-16 units, a hash, the UTF-16BE characters). The children of a
directory are sorted by name hash, since Qt finds a path component by binary search on it. `check()` finds every file back the way Qt's lookup does.
"""
import struct
import sys


def qt_hash(name):
    h = 0
    units = struct.unpack(">%dH" % (len(name.encode("utf-16-be")) // 2), name.encode("utf-16-be"))
    for c in units:
        h = ((h << 4) + c) & 0xFFFFFFFF
        h ^= (h & 0xF0000000) >> 23
        h &= 0x0FFFFFFF
    return h


class Node:
    def __init__(self, name, data=None):
        self.name = name
        self.data = data          # None for a directory
        self.children = {}


def build(files):
    root = Node("")
    for path, data in files.items():
        parts = [p for p in path.split("/") if p]
        node = root
        for p in parts[:-1]:
            node = node.children.setdefault(p, Node(p))
        node.children[parts[-1]] = Node(parts[-1], data)

    names, name_off = bytearray(), {}
    def name_of(n):
        if n not in name_off:
            name_off[n] = len(names)
            u = n.encode("utf-16-be")
            names.extend(struct.pack(">HI", len(u) // 2, qt_hash(n)) + u)
        return name_off[n]

    data_blob = bytearray()
    order = []                     # nodes in tree order: breadth first, children sorted by hash then name

    def sorted_children(n):
        return sorted(n.children.values(), key=lambda c: (qt_hash(c.name), c.name))

    queue = [root]
    index = {id(root): 0}
    while queue:
        n = queue.pop(0)
        order.append(n)
        for c in sorted_children(n):
            index[id(c)] = len(index)
            queue.append(c)

    tree = bytearray()
    for n in order:
        off = name_of(n.name)
        if n.data is None:
            kids = sorted_children(n)
            first = index[id(kids[0])] if kids else 0
            tree += struct.pack(">IHII", off, 2, len(kids), first)
        else:
            d = len(data_blob)
            data_blob += struct.pack(">I", len(n.data)) + n.data
            tree += struct.pack(">IHHHI", off, 0, 0, 1, d)      # flags 0 (not compressed), territory 0 (any), language 1 (the C locale, rcc's default: Qt's lookup only takes that or a match), data offset
        tree += struct.pack(">Q", 0)                            # last modified (none)
    header_len = 20
    tree_off = header_len
    data_off = tree_off + len(tree)
    names_off = data_off + len(data_blob)
    return b"qres" + struct.pack(">IIII", 2, tree_off, data_off, names_off) + bytes(tree) + bytes(data_blob) + bytes(names)


def check(blob, files):
    """Look every path up the way QResourceRoot::findNode does, from the bytes alone."""
    assert blob[:4] == b"qres"
    version, tree_off, data_off, names_off = struct.unpack(">IIII", blob[4:20])
    assert version == 2
    entry = 22

    def node(i):
        o = tree_off + i * entry
        name_o, flags = struct.unpack(">IH", blob[o:o + 6])
        a, b = struct.unpack(">II", blob[o + 6:o + 14]) if flags & 2 else (None, None)
        return name_o, flags, o

    def name_at(off):
        n, h = struct.unpack(">HI", blob[names_off + off:names_off + off + 6])
        s = blob[names_off + off + 6:names_off + off + 6 + 2 * n].decode("utf-16-be")
        return s, h

    def find(path):
        cur = 0
        for part in [p for p in path.split("/") if p]:
            _, flags, o = node(cur)
            assert flags & 2, "not a directory on the way to " + path
            count, first = struct.unpack(">II", blob[o + 6:o + 14])
            hit = None
            for k in range(first, first + count):          # linear here, but also assert the sort order Qt's binary search relies on
                no, _, _ = node(k)
                s, h = name_at(no)
                assert h == qt_hash(s)
                if s == part:
                    hit = k
            assert hit is not None, "no " + part + " in " + path
            ks = [name_at(node(k)[0])[1] for k in range(first, first + count)]
            assert ks == sorted(ks), "children not sorted by hash"
            cur = hit
        _, flags, o = node(cur)
        assert not flags & 2
        d = struct.unpack(">I", blob[o + 10:o + 14])[0]
        n = struct.unpack(">I", blob[data_off + d:data_off + d + 4])[0]
        return blob[data_off + d + 4:data_off + d + 4 + n]

    for path, data in files.items():
        assert find(path) == data, path


if __name__ == "__main__":
    out, specs = sys.argv[1], sys.argv[2:]
    files = {}
    for s in specs:
        path, src = s.split("=", 1)
        files[path] = open(src, "rb").read()
    blob = build(files)
    check(blob, files)
    open(out, "wb").write(blob)
    print("%s: %d bytes, %d file(s)" % (out, len(blob), len(files)))
