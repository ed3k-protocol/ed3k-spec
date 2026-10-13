# Reference tokeniser of Protocol.md §10.1 for the edge cases in ../vectors-v1.md.
# Needs the Unicode 16.0.0 tables (Python 3.14). Prints words and keywords per name and
# writes the distinct keywords, one per line, to the file given as argument.
import unicodedata as u, sys
UNSP = [(0x3005,0x3007),(0x3040,0x30FF),(0x31F0,0x31FF),(0x3400,0x4DBF),(0x4E00,0x9FFF),(0x20000,0x323AF),
        (0x0E00,0x0E7F),(0x0E80,0x0EFF),(0x1780,0x17FF),(0x1000,0x109F),(0xA9E0,0xA9FF),(0xAA60,0xAA7F)]
def unsp(c): return any(a <= ord(c) <= b for a, b in UNSP)
def lmn(c): return u.category(c)[0] in "LMN"
def norm(s):
    s = u.normalize("NFKC", u.normalize("NFKC", s).casefold())
    return "".join(c for c in s if u.category(c) != "Cf")
def split(s):
    out, w = [], ""
    for c in s + "\0":
        if lmn(c): w += c
        elif w: out += step3(w); w = ""
    return out
def step3(w):
    out, i = [], 0
    while i < len(w):
        j = i
        while j < len(w) and unsp(w[j]) == unsp(w[i]): j += 1
        run = w[i:j]
        if unsp(w[i]) and len(run) >= 2: out += [run[k:k+2] for k in range(len(run)-1)]
        else: out.append(run)
        i = j
    return out
def uniq(l):
    r = []
    for x in l:
        if x not in r: r.append(x)
    return r
def tok(name):
    s = norm(name)
    words = uniq(split(s))
    ext = []
    if "." in s:
        e = s.rsplit(".", 1)[1]
        if 1 <= len(e) <= 8 and all(u.category(c)[0] in "LN" for c in e): ext = split(e)
    kws = [w for w in words if len(w.encode()) >= 2 and w not in ext]
    return s, words, kws
names = ["The.Big_Sleep (1946) [1080p].mkv", "Ça-va_ÉTÉ.MP3", "Straße.ﬁnal.txt", "東京物語 Tokyo Story.avi",
 "Avi.Love.avi", "AC/DC", "a.b", "notes.", "archive.tar.backup123", "co­op.TXT", "été à la carte",
 "ＦＵＬＬ　Ｗｉｄｔｈ", "İstanbul", "x² ½", "don't", "abc東京物def.mkv", "x東y", "ｶﾞｰﾄﾞ", "映画.日本語", "น้ำ", "ab.x日本"]
esc = lambda s: "".join(c if 0x20 < ord(c) < 0x7f else "U+%04X " % ord(c) for c in s)
allk = []
for n in names:
    s, w, k = tok(n)
    print(repr(n), "| norm", [hex(ord(c)) for c in s] if any(ord(c) > 0x7e for c in s) else s)
    print("   words", w, [esc(x) for x in w if any(ord(c)>0x7e for c in x)])
    print("   keys ", k)
    allk += k
open(sys.argv[1], "w").write("\n".join(uniq(allk)) + "\n")
