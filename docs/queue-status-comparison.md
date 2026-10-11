# Waiting in the upload queue: who keeps a queued download alive

**Status:** design note (2026-10-11). Informative; the rules are in the main document.
**Relates to:** [`Protocol.md`](../Protocol.md) §12.1 and §12.5. Section numbers (§) refer to that document.

A downloader usually waits a long time in an uploader's queue before it gets a slot. This note
records how ed3k keeps that waiting cheap, compared with eMule and with the first draft of §12.1.

**Result:** the uploader decides per downloader. The few it expects to serve soon stay
connected and get a ping every 30 minutes. All others disconnect and come back after a time the
uploader sets, between 30 minutes and 4 hours. When a slot is free the uploader contacts the
downloader at once, connected or not.

Byte and packet figures are estimates from message sizes, not measurements.

## 1. The first draft and why it was dropped

The first draft kept one stream per (uploader, file) open for as long as the download was
wanted, with a keep-alive every 20 s, and had the uploader send a `FileStatus` on every change
of rank or part status.

- **Updates grow with the square of the queue.** One slot change moves the rank of everyone
  behind it, so every event causes one message per queued downloader.
- **Keep-alives cannot be stretched.** NAT mappings for UDP need traffic about every 20 to 30 s.
- **State per queued peer.** An uploader with a queue of 5,000 would hold 5,000 connections.
- **A downloader with many files** would hold one stream per file and source.

## 2. Comparison

| | eMule | First draft | ed3k §12.5 |
|---|---|---|---|
| While queued | no connection; a UDP reask every 29 min | open connection, keep-alive every 20 s | no connection for most; the uploader keeps a few connected |
| Who sets the pace | fixed by the protocol | — | the uploader, per downloader |
| Rank update | in the reask answer | on every change | at the next poll, or in the 30 min ping |
| Part status update | in the reask answer | on every completed piece | at the next poll or ping; every 5 min at most during a transfer |
| Slot free | uploader connects and accepts | already connected | `FileStatus` if connected, else dial and `SlotOffer` |
| Firewalled downloader | callback through server or buddy | already connected | hole punch through its relays |
| Several files from one peer | one requested file, one queue entry | one stream and entry per file | one requested file, one queue entry |
| Asking for another file | waiting time is kept | — | position is kept |
| Who ends the connection | either side | nobody while queued | the uploader, after telling the downloader when to come back |
| Entry removed | 1 h without reask | 1 h after disconnect | 30 min after the time to come back |

**Cost for an uploader with 5,000 queued downloaders**

| | Packets per second | Open connections |
|---|---|---|
| eMule | about 6 | 0 |
| First draft | about 500 | 5,000 |
| ed3k §12.5, 50 kept connected, the rest coming back after 2 h on average | about 5 for keep-alives, plus 0.7 polls per second | 50 |

A poll is a resumed QUIC connection of about 2 KB, against about 100 B for eMule's reask pair.
At 2 hours that is about 1.4 KB/s for the uploader above, against about 0.6 KB/s in eMule. The
interval is what keeps this in range, and it can be long because the poll no longer has to
discover a free slot.

## 3. What the poll is still for

Since the uploader calls the downloader when a slot is free, a poll only has to

- show that the downloader still wants the file,
- bring the downloader's current endpoints and relays, which the slot offer needs,
- refresh rank and part status for display and for piece selection.

None of these is urgent. That is why the uploader may set up to 4 hours, and why a downloader
comes back early only when its address or relays change.

## 4. Alternatives considered

- **Downloader always polls.** Simplest, and no state for queued peers. Every slot grant would
  need a dial back, also for the downloader that is next in line.
- **Connection stays, updates limited.** A slot starts without a dial, but the connections and
  keep-alives remain: about 80 times eMule's packet rate for a long queue.
- **A fixed interval of 30 minutes or 1 hour.** Fresher rank and part status, at several times
  the bytes and with no relief for long queues.
- **One request that lists several files**, with one queue entry per peer. One poll per peer
  instead of one per file, but larger messages and a file list per queued peer. ed3k keeps
  eMule's rule of one requested file per uploader.
- **Removing a downloader after one failed slot offer.** Hole punching fails often enough that
  firewalled downloaders would lose their place regularly.
- **A reask over sessionless datagrams**, in the manner of the Kad datagram binding
  ([`kad-datagram.md`](../extensions/kad-datagram.md)). It would bring a poll close to eMule's
  100 B. Not in version 1; it could be added later as a further binding for this one exchange.

## 5. Open points

- The number of downloaders an uploader keeps connected is local policy. A sensible default
  needs measurements of slot turnover.
- A downloader whose address changes while it is away is unreachable for a slot offer until it
  comes back. It keeps its entry, but the slot passes.
- The 5-minute limit on part status updates during a transfer is a first value. It decides how
  soon a downloader learns of pieces a partial source has completed.
