// SPDX-License-Identifier: MIT
//
// Interlock ERC-20 INTR Token Mint Platform
// 		(containing)
// components from OpenZeppelin v4.6.0 contract (token/ERC20/ERC20.sol)
//
// Contributors:
// blairmunroakusa
// ...

pragma solidity ^0.8.0;

import "./IERC20.sol";
import "./POOL.sol";

 /** derived from from oz:
 * functions should revert instead returning `false` on failure.
 * This behavior is nonetheless conventional and does not conflict
 * with the expectations of ERC20 applications.
 *
 * An {Approval} event is emitted on calls to {transferFrom}.
 * This allows applications to reconstruct the allowance for all accounts just
 * by listening to said events.
 *
 * Finally, the non-standard {decreaseAllowance} and {increaseAllowance}
 * functions have been added to mitigate the well-known issues around setting
 * allowances.
 **/

contract ERC20INTR is IERC20 {

	/** @dev **/

		// divisibility factor
	uint8 private _decimals = 18;
	uint256 private _DECIMAL = 10 ** _decimals;

		// pools
	string[12] private _poolNames = [
		"earlyvc",
		"ps1",
		"ps2",
		"ps3",
		"team",
		"ov",
		"advise",
		"reward",
		"founder",
		"partner",
		"white",
		"public" ];
	uint8 constant private _poolNumber = 12;

		// keeping track of pools
	struct PoolData {
		string name;
		uint256 tokens;
		uint8 payments;
		uint8 cliff;
		uint32 members; }
	PoolData[] private _pool;
	address[] private _pools;

		// keeping track of members
	struct MemberStatus {
		uint256 paid;
		uint256 share;
		address account;
		uint8 cliff;
		uint8 pool;
		uint8 payments; }
	MemberStatus[] private _members;

		// core token balance and allowance mappings
	mapping(address => uint256) private _balances;
	mapping(address => mapping(
		address => uint256)) private _allowances;

		// basic token data
	string private _name = "Interlock Network";
	string private _symbol = "INTR";
	uint256 private _totalSupply = 1000000000 * _DECIMAL;
	address private _owner;
	// decimals = 18 by default

		// tracking time
	uint256 public nextPayout;
	uint8 public monthsPassed; 

		// keeping track of irreversible actions
	bool public TGEtriggered = false;
	bool public supplySplit = false;
	
/*************************************************/

	/**
	* init
	**/

/*************************************************/

		 // owned by msg.sender
		// initializes contract
	constructor(
		uint256[_poolNumber] memory poolTokens_,
		uint8[_poolNumber] memory monthlyPayments_,
		uint8[_poolNumber] memory poolCliffs_,
		uint32[_poolNumber] memory poolMembers_
	) {
		_owner = msg.sender;
		_balances[address(this)] = 0; 

		for (uint8 i = 0; i < _poolNumber; i++) {
			poolTokens_[i] *= _DECIMAL;
			_pool.push(
				PoolData(
					_poolNames[i],
					poolTokens_[i],
					monthlyPayments_[i],
					poolCliffs_[i],
					poolMembers_[i] ) ); } }

/*************************************************/

	/**
	* modifiers
	**/

/*************************************************/

		// only allows owner to call
	modifier isOwner(
	) {
		require(msg.sender == _owner,
			"only owner can call");
		_; }

/*************************************************/

		// verifies zero address was not provied
	modifier noZero(
		address _address
	) {
		require(_address != address(0),
			"zero address where it shouldn't be");
		_; }

/*************************************************/

		// verifies there exists enough token to proceed
	modifier isEnough(
		uint256 _available,
		uint256 _amount
	) {
		require(_available >= _amount,
			"not enough tokens available");
		_; }

/*************************************************/

	/**
	* setup methods
	**/

/*************************************************/

		// creates account for each pool
	function splitSupply(
	) public isOwner {
		
		// guard
		require(supplySplit == false,
			"supply split already happened");

		// create pool accounts and initiate
		for (uint8 i = 0; i < _poolNumber; i++) {
			address Pool = address(new POOL());
			_pools.push(Pool);
			_balances[Pool] = 0;
			_allowances[address(this)][Pool] = 0; }

		// this must never happen again...
		supplySplit = true; }

/*************************************************/

		 // in csv tx batches by pool
		// allocates pool supply between members
	function addMembers (
		uint256[] calldata payouts,
		address[] calldata members,
		uint8[] calldata whichPool
	) public isOwner {

		// iterate through addMember input array
		for (uint8 i = 0; i < payouts.length; i++) {
			// intiate member  entry
			MemberStatus memory member;
			member.share = payouts[i] * _DECIMAL;
			member.cliff = _pool[whichPool[i]].cliff;
			member.payments = _pool[whichPool[i]].payments;
			member.account = members[i];
			member.pool = whichPool[i];
			_members.push(member); } }

/*************************************************/

		// generates all the tokens
	function triggerTGE(
	) public isOwner {

		// guards
		require(supplySplit == true,
			"supply not split");
		require(TGEtriggered == false,
			"TGE already happened");

		// mint
		_balances[address(this)] = _totalSupply;
		_approve(address(this), msg.sender, _totalSupply);
		emit Transfer(address(0), address(this), _totalSupply);

		// start the clock for time vault pools
		nextPayout = block.timestamp + 30 days;
		monthsPassed = 0;

		// apply the initial round of token distributions
		_poolDistribution();
		_memberDistribution();

		// this must never happen again...
		TGEtriggered = true; }

/*************************************************/

	/**
	* payout methods
	**/

/*************************************************/

		 // updates allowances and balances across pools and members
		// calls successfully after 30 days pass
	function distribute(
	) public isOwner {

		// guards
		require(_checkTime(), "too soon");

		// distribute tokens
		_poolDistribution();
		_memberDistribution(); }

/*************************************************/

		// distribute shares to all investor members
	function _memberDistribution(
	) internal {

		// iterate through members
		for (uint8 i = 0; i < _members.length; i++) {
			if (_members[i].cliff <= monthsPassed &&
				monthsPassed >= (_members[i].cliff + _members[i].payments)
				) {
				transferFrom(
					_pools[_members[i].pool],
					_members[i].account,
					_members[i].share );
				_members[i].paid += _members[i].share; } } }

/*************************************************/								
			
		// distribute tokens to pools on schedule
	function _poolDistribution(
	) internal {

		// iterate through pools
		for (uint8 i = 0; i < _poolNumber; i++) {
			if (_pool[i].cliff <= monthsPassed &&
				monthsPassed >= (_members[i].cliff + _members[i].payments)
				) {
				// transfer month's distribution to pools
				transferFrom(
					address(this),
					_pools[i],
					_pool[i].tokens/_pool[i].payments );
				_approve(
					_pools[i],
					msg.sender,
					_pool[i].tokens/_pool[i].payments ); } } }

/*************************************************/

		// makes sure that distributions do not happen too early
	function _checkTime(
	) internal returns (bool) {

		// test time
		if (block.timestamp > nextPayout) {
			nextPayout += 30 days;
			monthsPassed++;
			return true; }

		// not ready
		return true; }
			
/*************************************************/

		// renders contract as ownerLESS
	function disown(
	) public isOwner {

		//disown
		_owner = address(0); }

/*************************************************/

		// changes the contract owner
	function changeOwner(
		address newOwner
	) public isOwner {

		// reassign
		_owner = newOwner; }
		
/*************************************************/

	/**
	* getter methods
	**/

/*************************************************/

		// gets token name (Interlock Network)
	function name(
	) public view override returns (string memory) {
		return _name; }

/*************************************************/

		// gets token symbol (INTR)
	function symbol(
	) public view override returns (string memory) {
		return _symbol; }

/*************************************************/

		// gets token decimal number
	function decimals(
	) public view override returns (uint8) {
		return _decimals; }

/*************************************************/

		// gets tokens minted
	function totalSupply(
	) public view override returns (uint256) {
		return _totalSupply; }

/*************************************************/

		// gets account balance (tokens payable)
	function balanceOf(
		address account
	) public view override returns (uint256) {
		return _balances[account]; }

/*************************************************/

		// gets tokens spendable by spender from owner
	function allowance(
		address owner,
		address spender
	) public view virtual override returns (uint256) {
		return _allowances[owner][spender]; }

/*************************************************/

		// gets total tokens paid out in circulation
	function circulation() public view returns (uint256) {
		return _totalSupply - _balances[address(this)]; }

/*************************************************/

	/**
	* doer methods
	**/

/*************************************************/

		   // emitting Transfer, reverting on failure
		  // where caller balanceOf must be >= amount
		 // where `to` cannot = zero  address
		// increases spender allowance
	function transfer(
		address to,
		uint256 amount
	) public override returns (bool) {
		address owner = msg.sender;
		_transfer(owner, to, amount);
		return true; }

		// internal implementation of transfer() above
	function _transfer(
		address from,
		address to,
		uint256 amount
	) internal virtual noZero(from) noZero(to) isEnough(_balances[from], amount) {
		_beforeTokenTransfer(from, to, amount);
		unchecked {
			_balances[from] = _balances[from] - amount;}
		_balances[to] += amount;
		emit Transfer(from, to, amount);
		_afterTokenTransfer(from, to, amount); }

/*************************************************/

		  // emitting Approval, reverting on failure
		 // (=> no allownance delta when TransferFrom)
		// defines tokens available to spender from msg.sender
	function approve(
		address spender,
		uint256 amount
	) public override returns (bool) {
		address owner = msg.sender;
		_approve(owner, spender, amount);
		return true; }

		// internal implementation of approve() above 
	function _approve(
		address owner,
		address spender,
		uint256 amount
	) internal virtual noZero(owner) noZero(spender) {
		_allowances[owner][spender] = amount;
		emit Approval(owner, spender, amount); }

/*************************************************/

		     // emitting Approval, reverting on failure
		    // where msg.sender allowance w/`from` must be >= amount
		   // where `from` balance must be >= amount
		  // where `from` and `to` cannot = zero address
		 // which does not update allowance if allowance = u256.max
		// pays portion of spender's allowance with owner to recipient
	function transferFrom(
		address from,
		address to,
		uint256 amount
	) public override returns (bool) {
		address spender = msg.sender;
		_spendAllowance(from, spender, amount);
		_transfer(from, to, amount);
		return true; }

/*************************************************/

		  // emitting Approval, reverting on failure
		 // where `spender` cannot = zero address
		// atomically increases spender's allowance
	function increaseAllowance(
		address spender,
		uint256 addedValue
	) public returns (bool) {
		address owner = msg.sender;
		_approve(owner, spender, allowance(owner, spender) + addedValue);
		return true; }

 /* Above and below are alternatives to {approve} that can be used
 *  as a mitigation for problems described in {IERC20-approve}.
 *
 * ?? Why is there no owner balance check for increaseAllowance() ??
 **/
		   // emitting Approval, reverting on failure
		  // where `spender` must have allowance >= `subtractedValue`
		 // where `spender` cannot = zero address
		// atomically decreases spender's allowance
	function decreaseAllowance(
		address spender,
		uint256 amount
	) public isEnough(allowance(msg.sender, spender), amount) returns (bool) {
		address owner = msg.sender;
		unchecked {
			_approve(owner, spender, allowance(owner, spender) - amount);}
		return true; }

/*************************************************/

		   // emitting Transfer, reverting on failure
		  // where `account` must have >= burn amount
		 // where `account` cannot = zero address
		// decreases token supply by deassigning from account
	function _burn(
		address account,
		uint256 amount
	) internal noZero(account) isEnough(_balances[account], amount) {
		_beforeTokenTransfer(account, address(0), amount);
		unchecked {
			_balances[account] = _balances[account] - amount;}
		_totalSupply -= amount;
		emit Transfer(account, address(0), amount);
		_afterTokenTransfer(account, address(0), amount); }

/*************************************************/

		   // emitting Approval if finite, reverting on failure 
		  // will do nothing if infinite allowance
		 // used strictly internally
		// deducts from spender's allowance with owner
	function _spendAllowance(
		address owner,
		address spender,
		uint256 amount
	) internal isEnough(allowance(owner, spender), amount) {
		unchecked {
			_approve(owner, spender, allowance(owner, spender) - amount);}}

/*************************************************/

		    // where `from` && `to` != zero account => to be regular xfer
		   // where `from` = zero account => `amount` to be minted `to`
		  // where `to` = zero account => `amount` to be burned `from`
		 // where `from` && `to` = zero account => impossible
		// hook that inserts behavior prior to transfer/mint/burn
	function _beforeTokenTransfer(
		address from,
		address to,
		uint256 amount
	) internal virtual {}

/*************************************************/

		    // where `from` && `to` != zero account => was regular xfer
		   // where `from` = zero account => `amount` was minted `to`
		  // where `to` = zero account => `amount` was burned `from`
		 // where `from` && `to` = zero account => impossible
		// hook that inserts behavior prior to transfer/mint/burn
	function _afterTokenTransfer(
		address from,
		address to,
		uint256 amount
	) internal virtual {}

/*************************************************/

}



