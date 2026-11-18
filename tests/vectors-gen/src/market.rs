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
    let coupon = h("ed3k v1 market coupon", b"SAVE10");

    // rating key: Ed25519 seed byte i = 0x29 XOR i
    let rseed: Vec<u8> = (0..32u8).map(|i| 0x29 ^ i).collect();
    let rpk = SigningKey::from_bytes(rseed.as_slice().try_into().unwrap()).verifying_key().to_bytes().to_vec();

    // ORDER by the buyer (second key)
    let item = cat(&[&fb(1, &lhash), &fv(2, 1), &fb(6, &rpk)]);
    let order = cat(&[&fb(1, pk1), &fb(2, &lfile), &fm(3, &item), &fv(4, 1), &fb(5, b"BTC"), &fv(12, t), &fm(13, &price)]);
    let order_id = h("ed3k v1 market order", &order);
    let zero = [0u8; 32];
    let opre = cat(&[b"ed3k v1 market section", &zero, &[1u8], &order]);
    let osig = sign(seed2, &opre);
    let osec = cat(&[&fv(1, 1), &fb(2, &order), &fb(3, pk2), &fm(4, &sigmsg(&osig))]);

    // CONFIRM by the vendor: rating signature of item 0
    let rkpre = cat(&[b"ed3k v1 market rating key", &order_id, &0u32.to_be_bytes(), &rpk]);
    let rksig = sign(seed1, &rkpre);
    let confirm = cat(&[&fb(2, &rksig), &fm(3, &price)]);
    let cpre = cat(&[b"ed3k v1 market section", &order_id, &[2u8], &confirm]);
    let csig = sign(seed1, &cpre);
    let csec = cat(&[&fv(1, 2), &fb(2, &confirm), &fb(3, pk1), &fm(4, &sigmsg(&csig))]);
    let prev3 = h("ed3k v1 market chain", &cat(&[&order_id, &[2u8], &confirm]));

    // Market frame of the ORDER, buyer → store
    let mid: u64 = 0x1112131415161718;
    let frame = cat(&[&f64x(1, mid), &fb(2, pk2), &fb(3, pk1), &fb(4, &order_id), &fm(5, &osec)]);

    // rating file (§6), signed by the rating key
    let rating = cat(&[&fb(1, pk1), &fb(2, &order_id), &fb(4, &rksig), &fb(5, b"mug"), &fv(6, 5), &fb(11, b"good mug"), &fv(12, t + 86400)]);
    let rmagic = cat(&[b"ed3krat", &[1u8]]);
    let rpre = cat(&[b"ed3k v1 market doc", &rmagic, &rating]);
    let rsig = sign(&rseed, &rpre);
    let rfile = cat(&[&rmagic, &fb(1, &rating), &fb(2, &rpk), &fm(3, &sigmsg(&rsig))]);

    // moderator directory (§3.2)
    let dir = h("ed3k v1 topic", &cat(&[&[4u8], &zero, b"market.moderators"]))[..16].to_vec();

    println!("
## Market (market.md §2 to §4 and §6)

The store key (vendor) is the sample key; the buyer key is the second key. The rating key has
the Ed25519 seed byte *i* = `0x29 XOR i`. Prices are `{{ currency \"USD\", amount 1250,
divisibility 2 }}` = `{}`.

**Directory and coupon:**

```
market.moderators = H(\"ed3k v1 topic\", 04 ‖ 32 zero bytes ‖ \"market.moderators\")[0..16) = {}
coupon \"SAVE10\"   = H(\"ed3k v1 market coupon\", \"SAVE10\") = {}
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
`method` DIRECT, `coin` `BTC`, `issued` the sample time, `total` the price:

```
rating key  = {}
Item        = {}
Order       = {}
              ({} bytes)
order id    = H(\"ed3k v1 market order\", Order) = {}
signed      = \"ed3k v1 market section\" ‖ 32 zero bytes ‖ 01 ‖ Order      ({} bytes)
signature   = {}
Section     = {}
              ({} bytes)
```

**Market frame** carrying the `ORDER`, `id` `{mid:016x}`, from the buyer to the store:

```
Market      = {}
              ({} bytes)
```

**Confirm** by the vendor, `prev` = order id:

```
signed      = \"ed3k v1 market rating key\" ‖ order id ‖ 00000000 ‖ rating key      ({} bytes)
rating_sig  = {}
Confirm     = {}
signed      = \"ed3k v1 market section\" ‖ order id ‖ 02 ‖ Confirm      ({} bytes)
signature   = {}
Section     = {}
              ({} bytes)
next prev   = H(\"ed3k v1 market chain\", order id ‖ 02 ‖ Confirm) = {}
```

**Rating** of item 0, `overall` 5, `review` `good mug`, `issued` one day after the sample time,
signed by the rating key (market.md §6):

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
        hx(&rpk), hx(&item), w(&hx(&order), 14), order.len(), hx(&order_id), opre.len(), sig2(&osig), w(&hx(&osec), 14), osec.len(),
        w(&hx(&frame), 14), frame.len(),
        rkpre.len(), sig2(&rksig), w(&hx(&confirm), 14), cpre.len(), sig2(&csig), w(&hx(&csec), 14), csec.len(), hx(&prev3),
        w(&hx(&rating), 14), rating.len(), hx(&rmagic), rpre.len(), sig2(&rsig), w(&hx(&rfile), 14), rfile.len());
}
