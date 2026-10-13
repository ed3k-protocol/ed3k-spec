// SPDX-License-Identifier: LicenseRef-ed3k-public-domain
// Placed in the public domain, see ../LICENSE.
//
// Ledger of the ed3k credit extension (extensions/credit.md §6).
//
// NOT AUDITED, NOT TESTED, NOT DEPLOYED. NOT COMPILED since the holds (hold, holdFor,
// release, sweep, holdOf) were written by hand. The contract is immutable and has no
// owner: a fault cannot be fixed after deployment. It needs tests, fuzzing and
// an external audit first. The numbers marked (initial) were not simulated.
pragma solidity ^0.8.24;

/// Native minter of Subnet-EVM (precompile/contracts/nativeminter).
interface INativeMinter {
    function mintNativeCoin(address addr, uint256 amount) external;
}

/// The operator contract of the chain (credit.md §7.2). Not specified yet.
interface IOperators {
    /// True for the one credit address of each operator.
    function isRoot(address account) external view returns (bool);
}

contract CreditLedger {
    // ───────────────────────────── constants ─────────────────────────────

    string public constant name = "ed3k credit";
    string public constant symbol = "E3C";
    uint8 public constant decimals = 6;

    uint256 public constant UNIT = 1e6; // 1 credit = 1 MiB, in µ
    uint256 public constant FLOOR = 10 * UNIT; // earned credits stop decaying below this
    uint256 public constant GRACE = 3 days;
    uint256 public constant STEP = 1 days; // × 0.9 per step

    uint256 public constant MATURE_AGE = 30 days; // (initial)
    uint64 public constant MATURE_KIB = 1 << 20; // 1 GiB (initial)
    uint16 public constant MATURE_ISSUERS = 3; // (initial)
    uint64 public constant WEEK_CAP_KIB = 100 << 20; // 100 GiB per issuer and 7 days (initial)
    uint64 public constant PAIR_CAP_KIB = 10 << 20; // 10 GiB per issuer, holder and 7 days (initial)
    uint64 public constant MAX_RECEIPT_KIB = 50 << 20; // 50 GiB in a day (initial)
    uint32 public constant WINDOW_DAYS = 7; // (initial)
    uint256 public constant HOLD_TIME = 6 hours; // (initial): a bid's hold, credit.md §6.5
    uint256 public constant OVERDRAW_DAYS = 2; // (initial): a receipt older than this takes without the flag
    uint256 public constant TRANSFER_BURN_PCT = 10; // (initial)
    uint256 public constant UPLOADER_SHARE_PCT = 10; // of spent credits; the rest is burned
    uint256 public constant CALLER_REWARD_PCT = 60;

    INativeMinter public constant MINTER = INativeMinter(0x0200000000000000000000000000000000000001);

    bytes32 public constant RECEIPT_TYPEHASH =
        keccak256("Receipt(address issuer,address holder,uint32 day,uint64 kib,uint64 spent)");
    bytes32 public constant PERMIT_TYPEHASH =
        keccak256("Permit(address owner,address spender,uint256 value,uint256 nonce,uint256 deadline)");
    bytes32 public constant HOLD_TYPEHASH =
        keccak256("Hold(address payer,address payee,uint256 amount,uint256 nonce,uint256 deadline)");
    bytes32 private constant DOMAIN_TYPEHASH =
        keccak256("EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)");

    // secp256k1 order / 2; signatures with a higher s are refused
    uint256 private constant HALF_N = 0x7FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF5D576E7357A4501DDFE92F46681B20A0;

    // ───────────────────────── deployment parameters ─────────────────────

    IOperators public immutable operators;
    /// Native coin minted per receipt that did useful work (credit.md §6.6).
    uint256 public immutable gasReward;

    uint256 private immutable _chainId;
    bytes32 private immutable _domainSeparator;

    // ─────────────────────────────── state ───────────────────────────────

    struct Account {
        uint128 earned; // µ, from receipts
        uint128 received; // µ, from transfers and uploader shares
        uint32 lastActive; // day of the newest receipt that counted as activity
        uint64 decayedUntil; // decay is applied up to this time
        uint64 registeredAt; // first appearance
        uint64 inflow; // KiB credited for, from mature issuers
        uint16 issuers; // distinct mature issuers among them
        bool overdrawn;
    }

    struct AccountView {
        uint256 earned;
        uint256 received;
        uint32 lastActive;
        uint64 registeredAt;
        uint64 inflow;
        uint16 issuers;
        bool overdrawn;
        bool mature;
    }

    struct Receipt {
        address issuer;
        address holder;
        uint32 day;
        uint64 kib;
        uint64 spent;
    }

    struct Settled {
        uint64 kib; // KiB of this pair and day that minted, or were lost to a limit
        uint64 spent; // µ of this pair and day already taken
    }

    mapping(address => Account) private _accounts;
    /// issuer → holder → day
    mapping(address => mapping(address => mapping(uint32 => Settled))) public settled;
    /// issuer → week (day / 7) → KiB minted from this issuer
    mapping(address => mapping(uint32 => uint64)) public issuedKib;
    /// issuer → holder → week (day / 7) → KiB minted from this issuer to this holder
    mapping(address => mapping(address => mapping(uint32 => uint64))) public pairKib;
    /// holder → issuer → counted in Account.issuers
    mapping(address => mapping(address => bool)) private _countedIssuer;

    /// Supply as last written. An upper bound: decay not yet written back is still in it.
    uint256 public totalSupply;
    mapping(address => mapping(address => uint256)) public allowance;
    mapping(address => uint256) public nonces;

    // ─────────────────────────────── events ──────────────────────────────

    /// A bid's hold: `amount` of the payer's balance locked for `payee` until `until` (credit.md §6.5).
    struct Hold {
        uint128 amount;
        uint64 until;
    }

    /// payer → payee → hold
    mapping(address => mapping(address => Hold)) private _holds;
    /// payer → sum of its holds; a hold that ran out counts until it is released, swept or replaced
    mapping(address => uint256) private _heldTotal;
    /// payer → highest nonce used in holdFor
    mapping(address => uint256) public holdNonces;

    event Transfer(address indexed from, address indexed to, uint256 value);
    event Approval(address indexed owner, address indexed spender, uint256 value);
    event Registered(address indexed account);
    event Settle(address indexed issuer, address indexed holder, uint32 day, uint256 minted, uint256 taken);
    event Overdrawn(address indexed account, uint256 missing);
    event Held(address indexed payer, address indexed payee, uint256 amount, uint64 until);

    error BadDeadline();
    error BadNonce();
    error BadSignature();
    error HoldLocked();
    error IsOverdrawn();
    error InsufficientBalance();
    error InsufficientAllowance();
    error LengthMismatch();
    error ZeroAddress();

    // ───────────────────────────── construction ──────────────────────────

    constructor(IOperators operators_, uint256 gasReward_) {
        operators = operators_;
        gasReward = gasReward_;
        _chainId = block.chainid;
        _domainSeparator = _buildDomainSeparator();
    }

    // ────────────────────────────── settlement ───────────────────────────

    /// Anyone may call. A receipt that fails a check is skipped, not reverted,
    /// so one bad receipt does not cost the caller the batch.
    function settle(Receipt[] calldata receipts, bytes[] calldata sigs) external {
        if (receipts.length != sigs.length) revert LengthMismatch();
        uint32 today = uint32(block.timestamp / 1 days);
        for (uint256 i = 0; i < receipts.length; ++i) {
            _settleOne(receipts[i], sigs[i], today);
        }
    }

    /// Writes the decay of an account back and emits the burn.
    function poke(address account) external {
        _touch(_accounts[account], account);
    }

    // ──────────────────────────────── views ──────────────────────────────

    function balanceOf(address account) external view returns (uint256) {
        (uint256 e, uint256 r,) = _decayed(_accounts[account]);
        return e + r;
    }

    /// The account with decay applied as of now.
    function accountOf(address account) external view returns (AccountView memory v) {
        Account storage a = _accounts[account];
        (v.earned, v.received,) = _decayed(a);
        v.lastActive = a.lastActive;
        v.registeredAt = a.registeredAt;
        v.inflow = a.inflow;
        v.issuers = a.issuers;
        v.overdrawn = a.overdrawn;
        v.mature = isMature(account);
    }

    function isOverdrawn(address account) external view returns (bool) {
        return _accounts[account].overdrawn;
    }

    /// Whether receipts signed by `account` mint (credit.md §6.3).
    function isMature(address account) public view returns (bool) {
        if (_isRoot(account)) return true;
        Account storage a = _accounts[account];
        return a.registeredAt != 0 && block.timestamp >= uint256(a.registeredAt) + MATURE_AGE
            && a.inflow >= MATURE_KIB && a.issuers >= MATURE_ISSUERS;
    }

    function DOMAIN_SEPARATOR() public view returns (bytes32) {
        return block.chainid == _chainId ? _domainSeparator : _buildDomainSeparator();
    }

    function receiptDigest(Receipt calldata r) public view returns (bytes32) {
        return _digest(keccak256(abi.encode(RECEIPT_TYPEHASH, r.issuer, r.holder, r.day, r.kib, r.spent)));
    }

    // ─────────────────────────────── ERC-20 ──────────────────────────────
    // Differs from a plain ERC-20: balances fall with time without an event,
    // and a transfer delivers less than was sent (TRANSFER_BURN_PCT).

    function transfer(address to, uint256 value) external returns (bool) {
        _transfer(msg.sender, to, value);
        return true;
    }

    function transferFrom(address from, address to, uint256 value) external returns (bool) {
        uint256 allowed = allowance[from][msg.sender];
        if (allowed != type(uint256).max) {
            if (allowed < value) revert InsufficientAllowance();
            allowance[from][msg.sender] = allowed - value;
        }
        _transfer(from, to, value);
        return true;
    }

    function approve(address spender, uint256 value) external returns (bool) {
        allowance[msg.sender][spender] = value;
        emit Approval(msg.sender, spender, value);
        return true;
    }

    /// EIP-2612.
    function permit(address owner, address spender, uint256 value, uint256 deadline, uint8 v, bytes32 r, bytes32 s)
        external
    {
        if (block.timestamp > deadline) revert BadDeadline();
        bytes32 digest =
            _digest(keccak256(abi.encode(PERMIT_TYPEHASH, owner, spender, value, nonces[owner]++, deadline)));
        if (owner == address(0) || _recover(digest, v, r, s) != owner) revert BadSignature();
        allowance[owner][spender] = value;
        emit Approval(owner, spender, value);
    }

    /// Locks `amount` µ of the caller's balance for `payee` for HOLD_TIME (credit.md §6.5).
    /// While a hold runs it can only be raised or extended. Once it ran out it can be
    /// lowered, and 0 releases it.
    function hold(address payee, uint256 amount) external {
        _hold(msg.sender, payee, amount);
    }

    /// `hold` for a payer that sends no transaction: anyone may hand in its signature.
    /// `nonce` must be above every nonce used by this payer before. `sig` is 65 bytes, r ‖ s ‖ v.
    function holdFor(address payer, address payee, uint256 amount, uint256 nonce, uint256 deadline, bytes calldata sig)
        external
    {
        if (block.timestamp > deadline) revert BadDeadline();
        if (nonce <= holdNonces[payer]) revert BadNonce();
        bytes32 digest = _digest(keccak256(abi.encode(HOLD_TYPEHASH, payer, payee, amount, nonce, deadline)));
        if (payer == address(0) || _recoverSig(digest, sig) != payer) revert BadSignature();
        holdNonces[payer] = nonce;
        _hold(payer, payee, amount);
    }

    /// The payee gives up the hold of `payer`, at any time.
    function release(address payer) external {
        _dropHold(payer, msg.sender);
    }

    /// Anyone may remove a hold that ran out, so that it no longer counts against transfers.
    function sweep(address payer, address payee) external {
        if (block.timestamp < _holds[payer][payee].until) revert HoldLocked();
        _dropHold(payer, payee);
    }

    /// The hold of `payer` for `payee` as stored. It protects nothing once `until` has passed.
    function holdOf(address payer, address payee) external view returns (uint256 amount, uint64 until) {
        Hold storage h = _holds[payer][payee];
        return (h.amount, h.until);
    }

    // ────────────────────────────── internals ────────────────────────────

    function _settleOne(Receipt calldata r, bytes calldata sig, uint32 today) private {
        if (r.issuer == r.holder || r.issuer == address(0) || r.holder == address(0)) return;
        if (r.day > today || uint256(r.day) + WINDOW_DAYS < today) return;
        if (r.kib > MAX_RECEIPT_KIB) return;
        if (_recoverSig(receiptDigest(r), sig) != r.issuer) return;

        Settled storage st = settled[r.issuer][r.holder][r.day];
        if (r.kib < st.kib || r.spent < st.spent) return;

        Account storage issuer = _accounts[r.issuer];
        Account storage holder = _accounts[r.holder];
        _register(issuer, r.issuer);
        _register(holder, r.holder);

        // 1. take what was spent
        uint256 taken;
        if (r.spent > st.spent) {
            taken = _spend(issuer, holder, r.issuer, r.holder, uint256(r.spent) - st.spent, uint256(r.day) * 1 days);
            st.spent = r.spent;
        }

        // 2. mint what was delivered, 3. activity
        uint256 minted;
        if (r.kib > st.kib && !issuer.overdrawn && isMature(r.issuer)) {
            minted = _mint(holder, r, st);
        }

        if (minted != 0 || taken != 0) emit Settle(r.issuer, r.holder, r.day, minted, taken);

        // 4. gas reward
        if (minted >= UNIT || taken >= UNIT) _reward(msg.sender, r.holder);
    }

    function _register(Account storage a, address who) private {
        if (a.registeredAt == 0) {
            a.registeredAt = uint64(block.timestamp);
            emit Registered(who);
        }
    }

    /// Takes `amount` from the issuer, as far as it has it. Returns what was taken.
    /// A shortfall sets `overdrawn` only for a receipt at most OVERDRAW_DAYS old.
    function _spend(
        Account storage issuer,
        Account storage holder,
        address issuerAddr,
        address holderAddr,
        uint256 amount,
        uint256 dayStart
    ) private returns (uint256 taken) {
        _touch(issuer, issuerAddr);
        taken = _take(issuer, amount);
        // dayStart is not after now: the caller refused a day in the future
        if (taken < amount && block.timestamp / 1 days - dayStart / 1 days <= OVERDRAW_DAYS) {
            issuer.overdrawn = true;
            emit Overdrawn(issuerAddr, amount - taken);
        }
        if (taken == 0) return 0;
        _useHold(issuerAddr, holderAddr, taken);

        uint256 share = taken * UPLOADER_SHARE_PCT / 100;
        if (share != 0) {
            // decay is written up to the day of the receipt, not up to now: the activity of
            // step 3 then keeps the days from `day` on free of decay (credit.md §6.2)
            _touchAt(holder, holderAddr, dayStart > holder.decayedUntil ? dayStart : holder.decayedUntil);
            holder.received += uint128(share);
            emit Transfer(issuerAddr, holderAddr, share);
        }
        uint256 burned = taken - share;
        if (burned != 0) {
            totalSupply -= burned;
            emit Transfer(issuerAddr, address(0), burned);
        }
    }

    /// A payment uses up the hold it was placed for. No event: `Settle` carries what was taken.
    function _useHold(address payer, address payee, uint256 taken) private {
        Hold storage h = _holds[payer][payee];
        uint256 held = h.amount;
        if (held == 0) return;
        uint256 used = taken < held ? taken : held;
        h.amount = uint128(held - used);
        uint256 total = _heldTotal[payer];
        _heldTotal[payer] = total > used ? total - used : 0;
    }

    /// Rules of `hold` and `holdFor`. A hold that ran out counts as none: it can be lowered,
    /// and placing it again is a new hold.
    function _hold(address payer, address payee, uint256 amount) private {
        if (payee == address(0)) revert ZeroAddress();
        if (amount > type(uint128).max) revert InsufficientBalance();
        Hold storage h = _holds[payer][payee];
        uint256 old = h.amount;
        bool live = block.timestamp < h.until;
        if (live && amount < old) revert HoldLocked();
        uint256 total = _heldTotal[payer] - old + amount;
        if (amount != 0) {
            Account storage a = _accounts[payer];
            if (a.overdrawn) revert IsOverdrawn();
            // The balance is checked only when the hold grows; extending a running hold is free
            // of it, and so is a release, which must not fail (it would freeze transfers).
            if (!live || amount > old) {
                _touch(a, payer);
                if (uint256(a.earned) + a.received < total) revert InsufficientBalance();
            }
        }
        _heldTotal[payer] = total;
        h.amount = uint128(amount);
        h.until = amount == 0 ? 0 : uint64(block.timestamp + HOLD_TIME);
        emit Held(payer, payee, amount, h.until);
    }

    function _dropHold(address payer, address payee) private {
        Hold storage h = _holds[payer][payee];
        uint256 old = h.amount;
        if (old == 0 && h.until == 0) return;
        uint256 total = _heldTotal[payer];
        _heldTotal[payer] = total > old ? total - old : 0;
        h.amount = 0;
        h.until = 0;
        emit Held(payer, payee, 0, 0);
    }

    /// Whether `account` is an operator's root. A missing or reverting operator contract reads as false,
    /// so that it cannot revert a batch.
    function _isRoot(address account) private view returns (bool) {
        if (address(operators).code.length == 0) return false;
        try operators.isRoot(account) returns (bool root) {
            return root;
        } catch {
            return false;
        }
    }

    /// The caller has checked that the issuer is mature.
    function _mint(Account storage holder, Receipt calldata r, Settled storage st)
        private
        returns (uint256 minted)
    {
        uint32 week = r.day / 7;
        uint64 used = issuedKib[r.issuer][week];
        uint64 room = used < WEEK_CAP_KIB ? WEEK_CAP_KIB - used : 0;
        uint64 pairUsed = pairKib[r.issuer][r.holder][week];
        uint64 pairRoom = pairUsed < PAIR_CAP_KIB ? PAIR_CAP_KIB - pairUsed : 0;
        if (pairRoom < room) room = pairRoom;
        uint64 delta = r.kib - st.kib;
        uint64 grant = delta < room ? delta : room;

        // The value is computed on the cumulative count so that rounding does not add up.
        minted = _value(uint256(st.kib) + grant) - _value(st.kib);
        st.kib = r.kib; // what exceeds the limits is lost
        if (grant == 0) return 0;
        issuedKib[r.issuer][week] = used + grant;
        pairKib[r.issuer][r.holder][week] = pairUsed + grant;

        // Decay up to the day of the receipt is written first. Then the activity
        // counts, so that the days after it, up to the grace, do not decay.
        if (minted >= UNIT && r.day > holder.lastActive) {
            uint256 dayStart = uint256(r.day) * 1 days;
            _touchAt(holder, r.holder, dayStart < block.timestamp ? dayStart : block.timestamp);
            holder.lastActive = r.day;
        }
        _touch(holder, r.holder);

        holder.earned += uint128(minted);
        totalSupply += minted;
        emit Transfer(address(0), r.holder, minted);

        // contribution of the holder, for its own maturity
        unchecked {
            uint64 inflow = holder.inflow + grant;
            holder.inflow = inflow < holder.inflow ? type(uint64).max : inflow;
        }
        if (!_countedIssuer[r.holder][r.issuer]) {
            _countedIssuer[r.holder][r.issuer] = true;
            if (holder.issuers != type(uint16).max) ++holder.issuers;
        }
    }

    function _reward(address caller, address holder) private {
        if (gasReward == 0) return;
        if (caller == holder) {
            _mintCoin(holder, gasReward);
        } else {
            uint256 toCaller = gasReward * CALLER_REWARD_PCT / 100;
            _mintCoin(caller, toCaller);
            _mintCoin(holder, gasReward - toCaller);
        }
    }

    function _mintCoin(address to, uint256 amount) private {
        // Low-level, result ignored: a chain without the precompile, or one where
        // this contract is not allowed to mint, settles all the same.
        (bool ok,) = address(MINTER).call(abi.encodeCall(INativeMinter.mintNativeCoin, (to, amount)));
        ok;
    }

    function _transfer(address from, address to, uint256 value) private {
        if (to == address(0)) revert ZeroAddress();
        Account storage src = _accounts[from];
        if (src.overdrawn) revert IsOverdrawn();
        _touch(src, from);
        // a transfer never dips into what the account holds for its bids (credit.md §6.5)
        if (uint256(src.earned) + src.received < value + _heldTotal[from]) revert InsufficientBalance();
        if (_take(src, value) != value) revert InsufficientBalance();

        uint256 burned = value * TRANSFER_BURN_PCT / 100;
        uint256 delivered = value - burned;
        Account storage dst = _accounts[to];
        _touch(dst, to);
        dst.received += uint128(delivered);

        totalSupply -= burned;
        emit Transfer(from, to, delivered);
        if (burned != 0) emit Transfer(from, address(0), burned);
    }

    /// Takes up to `amount`, from `received` first. Returns what was taken.
    function _take(Account storage a, uint256 amount) private returns (uint256 taken) {
        uint256 r = a.received;
        uint256 e = a.earned;
        uint256 fromReceived = amount < r ? amount : r;
        uint256 rest = amount - fromReceived;
        uint256 fromEarned = rest < e ? rest : e;
        a.received = uint128(r - fromReceived);
        a.earned = uint128(e - fromEarned);
        return fromReceived + fromEarned;
    }

    /// Writes decay back.
    function _touch(Account storage a, address who) private {
        _touchAt(a, who, block.timestamp);
    }

    function _touchAt(Account storage a, address who, uint256 at) private {
        (uint256 e, uint256 r, uint64 until) = _decayedAt(a, at);
        uint256 lost = uint256(a.earned) + a.received - e - r;
        a.decayedUntil = until;
        if (lost != 0) {
            a.earned = uint128(e);
            a.received = uint128(r);
            totalSupply -= lost;
            emit Transfer(who, address(0), lost);
        }
    }

    /// Balances after decay, and the time up to which decay is then applied.
    /// A partial day is kept for later, so no day is lost or counted twice.
    function _decayed(Account storage a) private view returns (uint256 e, uint256 r, uint64 until) {
        return _decayedAt(a, block.timestamp);
    }

    function _decayedAt(Account storage a, uint256 at) private view returns (uint256 e, uint256 r, uint64 until) {
        e = a.earned;
        r = a.received;
        until = a.decayedUntil;
        uint256 start = uint256(a.lastActive) * 1 days + GRACE;
        uint256 from = until > start ? until : start;
        if (at <= from) return (e, r, until);

        uint256 steps = (at - from) / STEP;
        until = uint64(from + steps * STEP);
        // Ends when both amounts have stopped: at most a few hundred rounds for any uint128.
        while (steps != 0 && (e >= FLOOR || r != 0)) {
            if (e >= FLOOR) e = e * 9 / 10;
            if (r != 0) {
                r = r * 9 / 10;
                if (r < UNIT) r = 0;
            }
            --steps;
        }
    }

    /// µ for a cumulative count of KiB: 1 credit per MiB.
    function _value(uint256 kib) private pure returns (uint256) {
        return kib * 15625 / 16;
    }

    function _buildDomainSeparator() private view returns (bytes32) {
        return keccak256(
            abi.encode(DOMAIN_TYPEHASH, keccak256(bytes(name)), keccak256("1"), block.chainid, address(this))
        );
    }

    function _digest(bytes32 structHash) private view returns (bytes32) {
        return keccak256(abi.encodePacked("\x19\x01", DOMAIN_SEPARATOR(), structHash));
    }

    /// 65 bytes, r ‖ s ‖ v. Returns address(0) for a signature that is not valid.
    function _recoverSig(bytes32 digest, bytes calldata sig) private pure returns (address) {
        if (sig.length != 65) return address(0);
        bytes32 r = bytes32(sig[0:32]);
        bytes32 s = bytes32(sig[32:64]);
        uint8 v = uint8(sig[64]);
        return _recover(digest, v, r, s);
    }

    function _recover(bytes32 digest, uint8 v, bytes32 r, bytes32 s) private pure returns (address) {
        if (uint256(s) > HALF_N || (v != 27 && v != 28)) return address(0);
        return ecrecover(digest, v, r, s);
    }
}
