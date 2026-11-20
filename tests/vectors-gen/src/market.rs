// Market (market.md §2 to §4 and §6). Prints the section of ../vectors-v1.md as markdown.
use super::*;
use ext::w;

fn sig2(s: &[u8]) -> String { format!("{}\n              {}", hx(&s[..32]), hx(&s[32..])) }
fn f64x(f: u32, v: u64) -> Vec<u8> { let mut o = vec![]; if v == 0 { return o } vint(((f << 3) | 1) as u64, &mut o); o.extend(v.to_le_bytes()); o }

pub fn run(pk1: &[u8], seed1: &[u8], pk2: &[u8], seed2: &[u8]) {
    let sign = |seed: &[u8], m: &[u8]| { let sk = SigningKey::from_bytes(seed.try_into().unwrap()); let s = sk.sign(m); sk.verifying_key().verify_strict(m, &s).unwrap(); s.to_bytes().to_vec() };
    let t: u64 = 1791849600;
    let price = cat(&[&fb(1, b"USD"), &fv(2, 1250), &fv(3, 2)]);

    // listing file (§2.2, §2.3), store key = sample key
    let listing = cat(&[&fb(1, b"mug"), &fv(2, 1), &fb(3, b"Test mug"), &fm(8, &price), &fv(23, 1)]);
    let lmagic = cat(&[b"ed3klst", &[1u8]]);
    let lpre = cat(&[b"ed3k v1 market doc", &lmagic, &listing]);
    let lsig = sign(seed1, &lpre);
    let lfile = cat(&[&lmagic, &fb(1, &listing), &fb(2, pk1), &fm(3, &sigmsg(&lsig))]);
    let lhash = h("ed3k v1 market listing", &lfile);
    let coupon = h("ed3k v1 market coupon", &cat(&[pk1, b"SAVE10"]));

    // rating key: Ed25519 seed byte i = 0x29 XOR i
    let rseed: Vec<u8> = (0..32u8).map(|i| 0x29 ^ i).collect();
    let rpk = SigningKey::from_bytes(rseed.as_slice().try_into().unwrap()).verifying_key().to_bytes().to_vec();

    // a section (§4.2): signed over prev ‖ kind ‖ time ‖ body; next prev = H(chain, prev ‖ kind ‖ key ‖ time ‖ body)
    let section = |seed: &[u8], key: &[u8], prev: &[u8], kind: u8, time: u64, body: &[u8]| {
        let pre = cat(&[b"ed3k v1 market section", prev, &[kind], &time.to_be_bytes(), body]);
        let sig = sign(seed, &pre);
        let sec = cat(&[&fv(1, kind as u64), &fb(2, body), &fb(3, key), &fm(4, &sigmsg(&sig)), &fv(5, time)]);
        let next = h("ed3k v1 market chain", &cat(&[prev, &[kind], key, &time.to_be_bytes(), body]));
        (pre.len(), sig, sec, next)
    };

    // ORDER by the buyer (second key), DIRECT
    let item = cat(&[&fb(1, &lhash), &fv(2, 1), &fb(6, &rpk)]);
    let order = cat(&[&fb(1, pk1), &fb(2, &lfile), &fm(3, &item), &fv(4, 1), &fb(5, b"BTC"), &fv(12, t), &fm(13, &price)]);
    let order_id = h("ed3k v1 market order", &cat(&[pk2, &order]));
    let zero = [0u8; 32];
    let (olen, osig, osec, _) = section(seed2, pk2, &zero, 1, t, &order);

    // CONFIRM by the vendor, PAYMENT by the buyer, FULFILL by the vendor with the rating signature of item 0
    let confirm = cat(&[&fb(1, b"bc1qtestaddress"), &fm(3, &price)]);
    let (clen, csig, csec, p2) = section(seed1, pk1, &order_id, 2, t + 600, &confirm);
    let payment = cat(&[&fb(1, b"BTC"), &fb(2, b"txid-0001"), &fv(3, 25000)]);
    let (plen, psig, psec, p3) = section(seed2, pk2, &p2, 5, t + 1200, &payment);
    let rkpre = cat(&[b"ed3k v1 market rating key", &order_id, &0u32.to_be_bytes(), &lhash, &rpk]);
    let rksig = sign(seed1, &rkpre);
    let delivery = cat(&[&fb(1, b"Post"), &fb(2, b"TRACK1")]);
    let fulfill = cat(&[&fm(1, &delivery), &fb(4, &rksig)]);
    let (flen, fsig, fsec, p4) = section(seed1, pk1, &p3, 6, t + 3600, &fulfill);

    // Market frame of the ORDER, buyer → store
    let mid: u64 = 0x1112131415161718;
    let frame = cat(&[&f64x(1, mid), &fb(2, pk2), &fb(3, pk1), &fb(4, &order_id), &fm(5, &osec)]);

    // rating file (§6), signed by the rating key
    let rating = cat(&[&fb(1, pk1), &fb(2, &order_id), &fb(4, &rksig), &fb(5, b"mug"), &fv(6, 5), &fb(11, b"good mug"), &fv(12, t + 86400), &fb(15, &lhash)]);
    let rmagic = cat(&[b"ed3krat", &[1u8]]);
    let rpre = cat(&[b"ed3k v1 market doc", &rmagic, &rating]);
    let rsig = sign(&rseed, &rpre);
    let rfile = cat(&[&rmagic, &fb(1, &rating), &fb(2, &rpk), &fm(3, &sigmsg(&rsig))]);

    // moderator directory for BTC (§3.2)
    let dir = h("ed3k v1 topic", &cat(&[&[4u8], &zero, b"market.moderators.BTC"]))[..16].to_vec();

    println!("
## Market (market.md §2 to §4 and §6)

The store key (vendor) is the sample key; the buyer key is the second key. The rating key has
the Ed25519 seed byte *i* = `0x29 XOR i`. Prices are `{{ currency \"USD\", amount 1250,
divisibility 2 }}` = `{}`.

**Directory and coupon:**

```
market.moderators.BTC = H(\"ed3k v1 topic\", 04 ‖ 32 zero bytes ‖ \"market.moderators.BTC\")[0..16) = {}
coupon \"SAVE10\"       = H(\"ed3k v1 market coupon\", store key ‖ \"SAVE10\") = {}
```

**Listing** `{{ slug \"mug\", type PHYSICAL, title \"Test mug\", price, seq 1 }}` (market.md §2.2, §2.3):

```
Listing     = {}
signed      = \"ed3k v1 market doc\" ‖ {} ‖ Listing      ({} bytes)
signature   = {}
file        = magic ‖ MarketDoc {{ body, key, sigs }}
            = {}
              ({} bytes)
hash        = H(\"ed3k v1 market listing\", file) = {}
```

**Order** (market.md §4.2, §4.3). One item: the listing above, quantity 1, the rating key;
`method` DIRECT, `coin` `BTC`, `issued` = section `time` = the sample time, `total` the price:

```
rating key  = {}
Item        = {}
Order       = {}
              ({} bytes)
order id    = H(\"ed3k v1 market order\", buyer key ‖ Order) = {}
signed      = \"ed3k v1 market section\" ‖ 32 zero bytes ‖ 01 ‖ time ‖ Order      ({} bytes)
signature   = {}
Section     = {}
              ({} bytes)
```

**Market frame** carrying the `ORDER`, `id` `{mid:016x}`, from the buyer to the store:

```
Market      = {}
              ({} bytes)
```

**The chain on.** Each section signed over `\"ed3k v1 market section\" ‖ prev ‖ kind ‖ time ‖ body`;
the next `prev` = `H(\"ed3k v1 market chain\", prev ‖ kind ‖ key ‖ time ‖ body)`. `CONFIRM`
(vendor, sample time + 600, `chain` = `bc1qtestaddress`, `total` the price), `PAYMENT` (buyer,
+ 1,200, `coin` `BTC`, `txids` `txid-0001`, `amount` 25,000), `FULFILL` (vendor, + 3,600, one
`Delivery {{ carrier \"Post\", tracking \"TRACK1\" }}`, the rating signature of item 0):

```
CONFIRM     prev = order id
Confirm     = {}
signed      = ({} bytes)
signature   = {}
Section     = {}
              ({} bytes)

PAYMENT     prev = {}
Payment     = {}
signed      = ({} bytes)
signature   = {}
Section     = {}
              ({} bytes)

FULFILL     prev = {}
rating_sig  = Ed25519 by the vendor over \"ed3k v1 market rating key\" ‖ order id ‖ 00000000 ‖ listing hash ‖ rating key      ({} bytes)
            = {}
Fulfill     = {}
signed      = ({} bytes)
signature   = {}
Section     = {}
              ({} bytes)
next prev   = {}
```

**Rating** of item 0, `overall` 5, `review` `good mug`, `issued` one day after the sample time,
`listing` the listing hash, signed by the rating key (market.md §6):

```
Rating      = {}
              ({} bytes)
signed      = \"ed3k v1 market doc\" ‖ {} ‖ Rating      ({} bytes)
signature   = {}
file        = {}
              ({} bytes)
```",
        hx(&price), hx(&dir), hx(&coupon),
        hx(&listing), hx(&lmagic), lpre.len(), sig2(&lsig), w(&hx(&lfile), 14), lfile.len(), hx(&lhash),
        hx(&rpk), hx(&item), w(&hx(&order), 14), order.len(), hx(&order_id), olen, sig2(&osig), w(&hx(&osec), 14), osec.len(),
        w(&hx(&frame), 14), frame.len(),
        hx(&confirm), clen, sig2(&csig), w(&hx(&csec), 14), csec.len(),
        hx(&p2), hx(&payment), plen, sig2(&psig), w(&hx(&psec), 14), psec.len(),
        hx(&p3), rkpre.len(), sig2(&rksig), w(&hx(&fulfill), 14), flen, sig2(&fsig), w(&hx(&fsec), 14), fsec.len(), hx(&p4),
        w(&hx(&rating), 14), rating.len(), hx(&rmagic), rpre.len(), sig2(&rsig), w(&hx(&rfile), 14), rfile.len());
}
