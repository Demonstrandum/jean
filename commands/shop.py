#!/usr/bin/env python3
"""The shopping list (data/groceries.json), for the harness: shop.py add <text> [qty] | done <text> | remove <text> | clear-done | list
Matching is by the item's text, case-insensitive. The groceries feed picks a change up within seconds."""
import json, os, sys

PATH = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "data", "groceries.json")


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


def find(d, text):
    t = text.strip().lower()
    return next((i for i in d["items"] if i["text"].lower() == t), None)


def main(a):
    d = load()
    verb = a[0] if a else "list"
    if verb == "add":
        item = find(d, a[1])
        if item:
            item["done"] = False
            if len(a) > 2:
                item["qty"] = a[2]
        else:
            d["items"].append({"text": a[1].strip(), **({"qty": a[2]} if len(a) > 2 else {})})
        save(d)
        print("added", a[1])
    elif verb in ("done", "remove"):
        item = find(d, a[1])
        if not item:
            sys.exit(f"not on the list: {a[1]}")
        if verb == "done":
            item["done"] = True
        else:
            d["items"].remove(item)
        save(d)
        print(verb, a[1])
    elif verb == "clear-done":
        d["items"] = [i for i in d["items"] if not i.get("done")]
        save(d)
        print("cleared")
    elif verb == "list":
        for i in d["items"]:
            print(("x " if i.get("done") else "- ") + i["text"] + (f" ({i['qty']})" if i.get("qty") else ""))
    else:
        sys.exit("verbs: add <text> [qty] | done <text> | remove <text> | clear-done | list")


if __name__ == "__main__":
    main(sys.argv[1:])
