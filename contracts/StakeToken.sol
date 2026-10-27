// SPDX-License-Identifier: LicenseRef-ed3k-public-domain
// Placed in the public domain, see ../LICENSE.
//
// Stake token of the ed3k credit extension (extensions/credit.md §6.7).
//
// NOT AUDITED, NOT DEPLOYED. A plain ERC-20 with EIP-2612: no decay, no burn on transfer,
// no owner. Only the ledger that created it mints, for credits burned there.
pragma solidity ^0.8.24;

contract StakeToken {
    string public constant name = "ed3k stake";
    string public constant symbol = "E3S";
    uint8 public constant decimals = 18;

    bytes32 public constant PERMIT_TYPEHASH =
        keccak256("Permit(address owner,address spender,uint256 value,uint256 nonce,uint256 deadline)");
    bytes32 private constant DOMAIN_TYPEHASH =
        keccak256("EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)");
    // secp256k1 order / 2; signatures with a higher s are refused
    uint256 private constant HALF_N = 0x7FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF5D576E7357A4501DDFE92F46681B20A0;

    /// The ledger that created this token; the only account that mints.
    address public immutable minter;

    uint256 private immutable _chainId;
    bytes32 private immutable _domainSeparator;

    uint256 public totalSupply;
    mapping(address => uint256) public balanceOf;
    mapping(address => mapping(address => uint256)) public allowance;
    mapping(address => uint256) public nonces;

    event Transfer(address indexed from, address indexed to, uint256 value);
    event Approval(address indexed owner, address indexed spender, uint256 value);

    error NotMinter();
    error ZeroAddress();
    error InsufficientBalance();
    error InsufficientAllowance();
    error BadDeadline();
    error BadSignature();

    constructor() {
        minter = msg.sender;
        _chainId = block.chainid;
        _domainSeparator = _buildDomainSeparator();
    }

    /// Called by the ledger for a stake claim (credit.md §6.7).
    function mint(address to, uint256 value) external {
        if (msg.sender != minter) revert NotMinter();
        if (to == address(0)) revert ZeroAddress();
        totalSupply += value;
        unchecked {
            balanceOf[to] += value; // below totalSupply
        }
        emit Transfer(address(0), to, value);
    }

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
        bytes32 digest = keccak256(
            abi.encodePacked(
                "\x19\x01",
                DOMAIN_SEPARATOR(),
                keccak256(abi.encode(PERMIT_TYPEHASH, owner, spender, value, nonces[owner]++, deadline))
            )
        );
        if (owner == address(0) || _recover(digest, v, r, s) != owner) revert BadSignature();
        allowance[owner][spender] = value;
        emit Approval(owner, spender, value);
    }

    function DOMAIN_SEPARATOR() public view returns (bytes32) {
        return block.chainid == _chainId ? _domainSeparator : _buildDomainSeparator();
    }

    // ────────────────────────────── internals ────────────────────────────

    function _transfer(address from, address to, uint256 value) private {
        if (to == address(0)) revert ZeroAddress();
        uint256 bal = balanceOf[from];
        if (bal < value) revert InsufficientBalance();
        unchecked {
            balanceOf[from] = bal - value;
            balanceOf[to] += value; // sum of balances is totalSupply
        }
        emit Transfer(from, to, value);
    }

    function _buildDomainSeparator() private view returns (bytes32) {
        return keccak256(
            abi.encode(DOMAIN_TYPEHASH, keccak256(bytes(name)), keccak256("1"), block.chainid, address(this))
        );
    }

    function _recover(bytes32 digest, uint8 v, bytes32 r, bytes32 s) private pure returns (address) {
        if (uint256(s) > HALF_N || (v != 27 && v != 28)) return address(0);
        return ecrecover(digest, v, r, s);
    }
}
