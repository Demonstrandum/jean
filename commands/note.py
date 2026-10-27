#!/usr/bin/env python3
"""Notes (data/notes.json), for the harness: note.py add <text> [due] | done <text> | remove <text> | list"""
import json, os, sys

PATH = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "data", "notes.json")


def load():
    try:
        with open(PATH) as f:
            return json.load(f)
    except FileNotFoundError:
        return {"items": []}


def save(d):
    tmp = PATH + ".tmp"
    with open(tmp, "w") as f:
        json.dump(d, f, indent=2, ensure_ascii=False)
        f.write("\n")
    os.replace(tmp, PATH)


def main(a):
    d = load()
    verb = a[0] if a else "list"
    if verb == "add":
        d["items"].append({"text": a[1].strip(), **({"due": a[2]} if len(a) > 2 else {})})
        save(d)
        print("noted", a[1])
    elif verb in ("done", "remove"):
        t = a[1].strip().lower()
        item = next((i for i in d["items"] if i["text"].lower() == t), None)
        if not item:
            sys.exit(f"no such note: {a[1]}")
        if verb == "done":
            item["done"] = True
        else:
            d["items"].remove(item)
        save(d)
        print(verb, a[1])
    elif verb == "list":
        for i in d["items"]:
            print(("x " if i.get("done") else "- ") + i["text"] + (f" ({i['due']})" if i.get("due") else ""))
    else:
        sys.exit("verbs: add <text> [due] | done <text> | remove <text> | list")


if __name__ == "__main__":
    main(sys.argv[1:])
