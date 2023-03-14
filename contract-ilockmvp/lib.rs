//!
//! INTERLOCK NETWORK MVP SMART CONTRACT
//!  - PSP22 TOKEN
//!  - REWARDS
//!
//! This is a standard ERC20-style token contract
//! with provisions for enforcing a token distribution
//! vesting schedule, and for rewarding interlockers for
//! browsing the internet with the Interlock browser extension.
//!
//! #### To ensure build with cargo-contract version 2.0.0, run:
//!
//! cargo install cargo-contract --force --version 2.0.0
//!
//! #### To build, run:
//!
//! cargo +nightly contract build
//!
//! #### To build docs, run:
//!
//! cargo +nightly doc --no-deps --document-private-items --open
//!
//! #### To reroute docs in Github, run:
//!
//! echo "<meta http-equiv=\"refresh\" content=\"0; url=build_wheel\">" >
//! target/doc/index.html;
//! cp -r target/doc ./docs
//!

#![doc(
    html_logo_url = "https://user-images.githubusercontent.com/69293813/211380333-f29cd213-f1f5-46c6-8c02-5ba0e15588f0.png",
    html_favicon_url = "https://user-images.githubusercontent.com/69293813/211380333-f29cd213-f1f5-46c6-8c02-5ba0e15588f0.png",
)]

#![allow(non_snake_case)]
#![cfg_attr(not(feature = "std"), no_std)]
#![feature(min_specialization)]


pub use self::ilockmvp::{
    ILOCKmvp,
    ILOCKmvpRef,
};

#[openbrush::contract]
pub mod ilockmvp {

    use ink::{
        codegen::{EmitEvent, Env},
        reflect::ContractEventBase,
    };
    use ink::prelude::{
        vec::Vec,
        format,
        string::{String, ToString},
    };
    use ink::storage::Mapping;
    use openbrush::{
        contracts::{
            psp22::{
                extensions::{metadata::*, burnable::*},
                Internal,
            },
            ownable::*,
        },
        traits::Storage,
    };

////////////////////////////////////////////////////////////////////////////
//// constants /////////////////////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////

    /// - Magic numbers.
    pub const ID_LENGTH: usize = 32;                                // 32B account id
    pub const POOL_COUNT: usize = 12;                               // number of stakeholder pools
    pub const ONE_MONTH: Timestamp = 2_592_000_000;                 // milliseconds in 30 days

    /// - Token data.
    pub const TOKEN_CAP: u128 = 1_000_000_000;                      // 10^9
    pub const DECIMALS_POWER10: u128 = 1_000_000_000_000_000_000;   // 10^18
    pub const SUPPLY_CAP: u128 = TOKEN_CAP * DECIMALS_POWER10;      // 10^27
    pub const TOKEN_NAME: &str = "Interlock Network";
    pub const TOKEN_DECIMALS: u8 = 18;
    pub const TOKEN_SYMBOL: &str = "ILOCK";

    #[derive(Debug)]
    pub struct PoolData<'a> {
        name: &'a str,
        tokens: u128,
        vests: u8,
        cliffs: u8,
    }

    /// - Pool data.
    pub const POOLS: [PoolData; POOL_COUNT] = [
        PoolData { name: "early_backers+venture_capital", tokens: 20_000_000,  vests: 24, cliffs: 1, },
        PoolData { name: "presale_1",                     tokens: 48_622_222,  vests: 18, cliffs: 1, },
        PoolData { name: "presale_2",                     tokens: 66_666_667,  vests: 15, cliffs: 1, },
        PoolData { name: "presale_3",                     tokens: 40_000_000,  vests: 12, cliffs: 1, },
        PoolData { name: "team+founders",                 tokens: 200_000_000, vests: 36, cliffs: 6, },
        PoolData { name: "outlier_ventures",              tokens: 40_000_000,  vests: 24, cliffs: 1, },
        PoolData { name: "advisors",                      tokens: 25_000_000,  vests: 24, cliffs: 1, },
        PoolData { name: "rewards",                       tokens: 285_000_000, vests: 1,  cliffs: 0, },
        PoolData { name: "foundation",                    tokens: 172_711_111, vests: 84, cliffs: 1, },
        PoolData { name: "partners",                      tokens: 37_000_000,  vests: 1,  cliffs: 0, },
        PoolData { name: "whitelist",                     tokens: 15_000_000,  vests: 48, cliffs: 0, },
        PoolData { name: "public_sale",                   tokens: 50_000_000,  vests: 48, cliffs: 0, },
    ];

    /// - Pools.
    pub const EARLY_BACKERS: u8     = 0;
    pub const PRESALE_1: u8         = 1;
    pub const PRESALE_2: u8         = 2;
    pub const PRESALE_3: u8         = 3;
    pub const TEAM_FOUNDERS: u8     = 4;
    pub const OUTLIER_VENTURES: u8  = 5;
    pub const ADVISORS: u8          = 6;
    pub const REWARDS: u8           = 7;
    pub const FOUNDATION: u8        = 8;
    pub const PARTNERS: u8          = 9;
    pub const WHITELIST: u8         = 10;
    pub const PUBLIC_SALE: u8       = 11;

////////////////////////////////////////////////////////////////////////////
//// structured data ///////////////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////

    /// - This is upgradable storage for the token rewarding feature of this
    /// PSP22 contract.
    pub const REWARD_KEY: u32 = openbrush::storage_unique_key!(RewardData);
    #[derive(Default, Debug)]
    #[openbrush::upgradeable_storage(REWARD_KEY)]
    pub struct RewardData {

        // ABSOLUTELY DO NOT CHANGE THE ORDER OF THESE VARIABLES
        // OR TYPES IF UPGRADING THIS CONTRACT!!!

        /// - How much ILOCK have we rewarded each Interlocker?
        interlocker: Mapping<AccountId, Balance>,

        /// - In total, how much ILOCK have we rewarded to Interlockers?
        total: Balance,

        /// - Expand storage related to the pool accounting functionality.
        pub _reserved: Option<()>,
    }

    /// - This is upgradable storage for the token pool management and accounting feature of this
    /// PSP22 contract.
    pub const POOL_KEY: u32 = openbrush::storage_unique_key!(TokenPools);
    #[derive(Default, Debug)]
    #[openbrush::upgradeable_storage(POOL_KEY)]
    pub struct TokenPools {

        // ABSOLUTELY DO NOT CHANGE THE ORDER OF THESE VARIABLES
        // OR TYPES IF UPGRADING THIS CONTRACT!!!

        /// - What are the current balances of all the vesting pools?
        /// - This includes the rewards pool balance.
        balances: [Balance; POOL_COUNT],

        /// - How much ILOCK is circulating right now?
        /// - This includes token held by liquidity pools/exchanges.
        /// - This is the value of `total_supply()` getter.
        circulating: Balance,

        /// - How much do we have available in collected taxes/fees from port owners
        /// and application contract operators?
        proceeds: Balance,

        /// - Expand storage related to the pool accounting functionality.
        pub _reserved: Option<()>,
    }

    /// - This is upgradable storage for the application connection feature of this
    /// PSP22 contract (ie, the application/socket/port contract connectivity formalism).
    pub const VEST_KEY: u32 = openbrush::storage_unique_key!(VestData);
    #[derive(Default, Debug)]
    #[openbrush::upgradeable_storage(VEST_KEY)]
    pub struct VestData {

        // ABSOLUTELY DO NOT CHANGE THE ORDER OF THESE VARIABLES
        // OR TYPES IF UPGRADING THIS CONTRACT!!!

        /// - Contains information about stakeholders and the vesting
        /// status.
        /// - See detailed struct below.
        ///
        /// stakeholder:         stakeholder account address -> info about stakeholder
        pub stakeholder: Mapping<AccountId, StakeholderData>,

        /// - Counter responsible for keeping track of how many months have passed
        /// along the vesting schedule.
        /// - Used in part to calculate and compare token amount paid out vs token amount owed.
        pub monthspassed: u16,

        /// - Stores the date timestamp one month ahead of the last increment of
        /// `monthspassed`
        pub nextpayout: Timestamp,

        /// - Expand storage related to the vesting functionality.
        pub _reserved: Option<()>,
    }
    /// - StakeholderData struct contains all pertinent information for each stakeholder
    /// (Besides balance and allowance mappings).
    /// - This is primarily for managing and implementing the vesting schedule.
    #[derive(scale::Encode, scale::Decode, Clone, Default)]
    #[cfg_attr(
    feature = "std",
    derive(
        Debug,
        PartialEq,
        Eq,
        scale_info::TypeInfo,
        ink::storage::traits::StorageLayout
        )
    )]
    pub struct StakeholderData {

        // ABSOLUTELY DO NOT CHANGE THE ORDER OF THESE VARIABLES
        // OR TYPES IF UPGRADING THIS CONTRACT!!!

        /// - How much so far has this stakeholder been paid in ILOCK?
        paid: Balance,

        /// - What is the overall ILOCK token share for this stakeholder?
        share: Balance,

        /// - Which vesting pool does this stakeholder belong to?
        /// - The pool determines the vesting schedule.
        pool: u8,
    }

    /// - This is upgradable storage for the application connection feature of this
    /// PSP22 contract (ie, the application/socket/port contract connectivity formalism).
    pub const APP_KEY: u32 = openbrush::storage_unique_key!(ApplicationData);
    #[derive(Default, Debug)]
    #[openbrush::upgradeable_storage(APP_KEY)]
    pub struct AppData {

        // ABSOLUTELY DO NOT CHANGE THE ORDER OF THESE VARIABLES
        // OR TYPES IF UPGRADING THIS CONTRACT!!!

        /// - Contains information specifying a particular _type_ of connecting
        /// external application contract via the application/socket abstraction.
        /// - When an application contract creates a connecting socket with this token
        /// contract with a particular port, it adheres to the logic and protocol
        /// specified by the port type.
        /// - For example, PORT 0 in this contract only accepts connections from universal
        /// access NFT contract owned by Interlock, and for every socket call from a UANFT contract 
        /// application, tokens in the amount of the set NFT price are transferred from the calling minter
        /// to this ILOCK contract's owner account. On the application side, once the ILOCK
        /// tokens are successfully transferred via the port protocol, a UANFT is minted to
        /// the caller.
        /// - For example, PORT 1 in this contract is the same as PORT 0, but UANFT application
        /// contracts are owned by different operators, and on each socket call, the protocol
        /// includes an additional tax in ILOCK, which Interlock Network collects.
        /// - The mapping is from port number, to port details and specs.
        /// - Only this contract's owner has the authority to create or edit a port.
        /// - See detailed struct below.
        ///
        /// ports:         port number -> port(app contract hash, metadata, port owner)
        ///
        pub ports: Mapping<u16, Port>,

        /// - Contains information specifying a particular _instance_ of an application
        /// (as defined by port application hash) contract's connection to this PSP22
        /// contract.
        /// - Similar to the standard TCP/IP address:port format, the port specifies the
        /// protocol, and the address specifies the operator of that particular instance
        /// of the application contract connecting to this PSP22 contract.
        /// - In the example of PORT 1, the address of a socket connection is the address
        /// that receives the ILOCK token transfer, ultimately in exchange for the UANFT
        /// mint back on the application side.
        /// - The mapping is from application address, to socket operator address and port number.
        /// - One socket may serve multiple applications (ie, the same operator address:port
        /// number pair) which is a slight deviation from the socket formality in TCP/IP.
        /// - Any agent with a verified application contract may connect to this PSP22 contract
        /// without permission from this contract's owner.
        /// - See detailed struct below.
        ///
        /// sockets:         application contract address -> socket(app operator address : port)
        ///
        pub sockets: Mapping<AccountId, Socket>,

        /// - Expand storage related to the application/socket/port functionality.
        pub _reserved: Option<()>,
    }
    /// - Information pertaining to port definition in application/socket/port contract
    /// connectivity formalism.
    #[derive(scale::Encode, scale::Decode, Clone)]
    #[cfg_attr(
    feature = "std",
    derive(
        Debug,
        PartialEq,
        Eq,
        scale_info::TypeInfo,
        ink::storage::traits::StorageLayout
        )
    )]
    pub struct Port {

        // ABSOLUTELY DO NOT CHANGE THE ORDER OF THESE VARIABLES
        // OR TYPES IF UPGRADING THIS CONTRACT!!!

        /// - What is the codehash of the application smart contract associated with
        /// this port?
        /// - This codehash is the application template that numerous individual application 
        /// contracts may be instantiated and connected to this PSP22 contract via socket
        /// without signed permission from this ILOCK contract's owner.
        /// - This codehash is essential to making sure that only safe and approved application
        /// contracts are able to connect to this token contract and manipulate its owneronly
        /// functionalities (as defined per respective port protocol).
        application: Hash,

        /// - How much does Interlock tax transaction taking place within a port protocol's
        /// socket call?
        tax: Balance,

        /// - For withdrawing rewards from ILOCK rewards pool, what is the max this particular
        /// port owner's application type can withdraw from rewards pool?
        cap: Balance,

        /// - If locked, only Interlock token contract owner can create a socket connection with
        /// this token contract using the appropriate application codehash.
        locked: bool,

        /// - How much ILOCK has this port been rewarded or issued throughout the course of
        /// its operation (in case where protocol rewards or issues ILOCK, that is)?
        paid: Balance,

        /// - How much has Interlock collected from this port in taxes or other collections?
        collected: Balance,

        /// - Who is the overall owner of this port?
        /// - Socket operators are not necessarily owners of the port.
        /// - For example, a restaurant franchise has one owner, whereas the franchise may have
        /// numberous restaurant locations, each with it's own operator, each operator/franchise
        /// pair forming a separate socket connection.
        owner: AccountId,
    }
    /// - Ink 4 has no AccountId Default impl thus struct Default cannot be derived
    /// due to `owner` field.
    /// - Default derivation is required by openbrush contract implementation of
    /// contract storage.
    impl Default for Port {
        fn default() -> Port {
            Port {
                application: Default::default(),
                tax: 0,
                cap: 0,
                locked: true,
                paid: 0,
                collected: 0,
                owner: AccountId::from([1_u8;32]),
            }
        }
    }
    /// - Information pertaining to socket definition in application/socket/port contract
    /// connectivity formalism.
    #[derive(scale::Encode, scale::Decode, Clone, Copy)]
    #[cfg_attr(
    feature = "std",
    derive(
        Debug,
        PartialEq,
        Eq,
        scale_info::TypeInfo,
        ink::storage::traits::StorageLayout
        )
    )]
    pub struct Socket {

        // ABSOLUTELY DO NOT CHANGE THE ORDER OF THESE VARIABLES
        // OR TYPES IF UPGRADING THIS CONTRACT!!!

        /// - Who operates (owns usually) a specific instance of a connecting application
        /// contract?
        /// - Using the restaurant franchise metaphor again, the operator may have several
        /// different instances of the port's application contract.
        /// - Each instance of the application contract has its own address, but each restaurant
        /// has the same operator.
        /// - The socket (operator:franchise or operator:port#) is like the single business franchise
        /// agreement between the restaurant operator and the franchise owner.
        /// - There is only one agreement between the franchise and the restaurant operator,
        /// regardless of how many restaurants the operator has.
        operator: AccountId,

        /// - What port is this operator connected to?
        /// - Using the restaurant franchise metaphor again, the port is like the franchise
        /// itself.
        /// - The port number is what identifies a particular franchise and its protocols,
        /// procedures, metadata, and ultimately business model and standards for any
        /// franchisees.
        portnumber: u16,
    }
    /// - Ink 4 has no AccountId Default impl thus struct Default cannot be derived
    /// due to `operator` field.
    impl Default for Socket {
        fn default() -> Socket {
            Socket {
                operator: AccountId::from([1_u8;32]),
                portnumber: 65535,
            }
        }
    }

    /// - ILOCKmvp struct contains overall storage data for contract
    #[ink(storage)]
    #[derive(Default, Storage)]
    pub struct ILOCKmvp {

        // ABSOLUTELY DO NOT CHANGE THE ORDER OF THESE VARIABLES
        // OR TYPES IF UPGRADING THIS CONTRACT!!!

        /// - Openbrush PSP22.
        #[storage_field]
        psp22: psp22::Data,

        /// - Openbrush ownership extension.
        #[storage_field]
        ownable: ownable::Data,

        /// - Openbrush metadata extension.
        #[storage_field]
        metadata: metadata::Data,

        /// - ILOCK Rewards info.
        #[storage_field]
        reward: RewardData,

        /// - ILOCK token pool info.
        #[storage_field]
        pool: TokenPools,

        /// - ILOCK vesting info.
        #[storage_field]
        vest: VestData,

        /// - ILOCK connecting application contract info
        #[storage_field]
        app: AppData,
    }

////////////////////////////////////////////////////////////////////////////
//// events and errors /////////////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////

    /// - Specify transfer event.
    #[ink(event)]
    pub struct Transfer {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        amount: Balance,
    }

    /// - Specify approval event.
    #[ink(event)]
    pub struct Approval {
        #[ink(topic)]
        owner: Option<AccountId>,
        #[ink(topic)]
        spender: Option<AccountId>,
        amount: Balance,
    }

    /// - Specify reward event.
    #[ink(event)]
    pub struct Reward {
        #[ink(topic)]
        to: Option<AccountId>,
        amount: Balance,
    }

    /// - Other contract error types.
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo)
    )]
    pub enum OtherError {
        /// - Returned if caller is not contract owner.
        CallerNotOwner,
        /// - Returned if stakeholder share is entirely paid out.
        StakeholderSharePaid,
        /// - Returned if the stakeholder doesn't exist.
        StakeholderNotFound,
        /// - Returned if stakeholder has not yet passed cliff.
        CliffNotPassed,
        /// - Returned if it is too soon to payout for month.
        PayoutTooEarly,
        /// - Returned if reward is too large.
        PaymentTooLarge,
        /// - Returned if socket does not exist.
        NoSocket,
        /// - Returned if port does not exist.
        NoPort,
        /// - Returned if not contract.
        NotContract,
        /// - Returned if only owner can add socket.
        PortLocked,
        /// - Returned if port cap is surpassed.
        PortCapSurpassed,
        /// - Returned if reward recipient is a contract.
        CannotRewardContract,
        /// - Returned if socket contract does not match registered hash.
        UnsafeContract,
        /// - Returned if application contract caller is not its operator.
        CallerNotOperator,
        /// - Returned if checked add overflows.
        Overflow,
        /// - Returned if checked sub underflows.
        Underflow,
        /// - Returned if checked divide errors out.
        DivError,
        /// - Returned if share is not greater than zero.
        ShareTooSmall,
        /// - Returned if pool number provided is invalid.
        InvalidPool,
        /// - Returned if port number provided is invalid.
        InvalidPort,
        /// - Custom contract error.
        Custom(String),
    }

    /// - Convert from OtherError into PSP22Error.
    impl Into<PSP22Error> for OtherError {
        fn into(self) -> PSP22Error {
            PSP22Error::Custom(format!("{:?}", self).into_bytes())
        }
    }

    /// - Convert from PSP22Error into OtherError.
    impl Into<OtherError> for PSP22Error {
        fn into(self) -> OtherError {
            OtherError::Custom(format!("{:?}", self))
        }
    }

    /// - For ILOCKmvpRef used in PSP34 or application contracts.
    impl From<OwnableError> for OtherError {
        fn from(error: OwnableError) -> Self {
            OtherError::Custom(format!("{:?}", error))
        }
    }

    /// - Convenience Result Type.
    pub type PSP22Result<T> = core::result::Result<T, PSP22Error>;

    /// - Convenience Result Type
    pub type OtherResult<T> = core::result::Result<T, OtherError>;

    /// - Needed for Openbrush internal event emission implementations.
    pub type Event = <ILOCKmvp as ContractEventBase>::Type;

////////////////////////////////////////////////////////////////////////////
/////// reimplement some functions /////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////

    impl PSP22 for ILOCKmvp {
        
        /// - Override default total_supply getter.
        /// - Total supply reflects token in circulation.
        #[ink(message)]
        fn total_supply(&self) -> Balance {

            // revert, testing set code hash
            self.pool.circulating
        }

        /// - Override default transfer doer.
        /// - Transfer from owner increases total circulating supply.
        /// - Transfer to owner decreases total circulating supply.
        #[ink(message)]
        fn transfer(
            &mut self,
            to: AccountId,
            value: Balance,
            data: Vec<u8>,
        ) -> PSP22Result<()> {

            let from = self.env().caller();

            let _ = self._transfer_from_to(from, to, value, data)?;

            // if sender is owner, then tokens are entering circulation
            if from == self.ownable.owner {

                match self.pool.circulating.checked_add(value) {
                    Some(sum) => self.pool.circulating = sum,
                    None => return Err(OtherError::Overflow.into()),
                };
            }

            // if recipient is owner, then tokens are being returned or added to rewards pool
            if to == self.ownable.owner {

                match self.pool.balances[REWARDS as usize].checked_add(value) {
                    Some(sum) => self.pool.balances[REWARDS as usize] = sum,
                    None => return Err(OtherError::Overflow.into()),
                };
                match self.pool.circulating.checked_sub(value) {
                    Some(difference) => self.pool.circulating = difference,
                    None => return Err(OtherError::Underflow.into()),
                };
            }

            Ok(())
        }

        /// - Override default transfer_from_to doer.
        /// - Transfer from owner increases total supply.
        #[ink(message)]
        fn transfer_from(
            &mut self,
            from: AccountId,
            to: AccountId,
            value: Balance,
            data: Vec<u8>,
        ) -> PSP22Result<()> {

            let caller = self.env().caller();
            let allowance = self._allowance(&from, &caller);

            let _ = self._approve_from_to(from, caller, allowance - value)?;
            let _ = self._transfer_from_to(from, to, value, data)?;

            // if sender is owner, then tokens are entering circulation
            if from == self.ownable.owner {

                match self.pool.circulating.checked_add(value) {
                    Some(sum) => self.pool.circulating = sum,
                    None => return Err(OtherError::Overflow.into()),
                };
            }

            // if recipient is owner, then tokens are being returned or added to rewards pool
            if to == self.ownable.owner {

                match self.pool.balances[REWARDS as usize].checked_add(value) {
                    Some(sum) => self.pool.balances[REWARDS as usize] = sum,
                    None => return Err(OtherError::Overflow.into()),
                };
                match self.pool.circulating.checked_sub(value) {
                    Some(difference) => self.pool.circulating = difference,
                    None => return Err(OtherError::Underflow.into()),
                };
            }

            Ok(())
        }
    }

    impl PSP22Metadata for ILOCKmvp {}

    impl Ownable for ILOCKmvp {
        
        // PRIOR TO OWNER TRANSFER,
        // REMAINING OWNER NONCIRCULATING
        // BALANCE MUST BE TRANSFERRED TO NEW OWNER.
    }

    impl PSP22Burnable for ILOCKmvp {

        /// - Override default burn doer.
        /// - Burn function to permanently remove tokens from circulation / supply.
        #[ink(message)]
		#[openbrush::modifiers(only_owner)]
        fn burn(
            &mut self,
            donor: AccountId,
            amount: Balance,
        ) -> PSP22Result<()> {

            // burn the tokens
            let _ = self._burn_from(donor, amount)?;

            // adjust pool balances
            if donor == self.ownable.owner {
                match self.pool.balances[REWARDS as usize].checked_sub(amount) {
                    Some(difference) => self.pool.balances[REWARDS as usize] = difference,
                    None => return Err(OtherError::Underflow.into()),
                };
            } else {
                match self.pool.circulating.checked_sub(amount) {
                    Some(difference) => self.pool.circulating = difference,
                    None => return Err(OtherError::Underflow.into()),
                };
            }

            Ok(())
        }
	}

    impl Internal for ILOCKmvp {

        /// - Impliment Transfer emit event because Openbrush doesn't.
        fn _emit_transfer_event(
            &self,
            _from: Option<AccountId>,
            _to: Option<AccountId>,
            _amount: Balance,
        ) {
            ILOCKmvp::emit_event(
                self.env(),
                Event::Transfer(Transfer {
                    from: _from,
                    to: _to,
                    amount: _amount,
                }),
            );
        }

        /// - Impliment Approval emit event because Openbrush doesn't.
        fn _emit_approval_event(
            &self,
            _owner: AccountId,
            _spender: AccountId,
            _amount: Balance
        ) {
            ILOCKmvp::emit_event(
                self.env(),
                Event::Approval(Approval {
                    owner: Some(_owner),
                    spender: Some(_spender),
                    amount: _amount,
                }),
            );
        }
    }

    /// - This is for linking openbrush PSP34 or application contract.
    impl Default for ILOCKmvpRef {
        fn default() -> ILOCKmvpRef {
            ink::env::call::FromAccountId::from_account_id(AccountId::from([1_u8; 32]))
        }
    }

////////////////////////////////////////////////////////////////////////////
/////// implement token contract ///////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////

    impl ILOCKmvp {

        /// - Function for internal _emit_event implementations.
        pub fn emit_event<EE: EmitEvent<Self>>(emitter: EE, event: Event) {
            emitter.emit_event(event);
        }

        /// - Constructor to initialize contract.
        #[ink(constructor)]
        pub fn new_token(
        ) -> Self {

            // create contract
            let mut contract = Self::default();
                
            // define owner as caller
            let caller = contract.env().caller();

            // set initial data
            contract.vest.monthspassed = 0;
            contract.vest.nextpayout = Self::env().block_timestamp() + ONE_MONTH;
            contract.reward.total = 0;
            contract.pool.circulating = 0;

            contract.metadata.name = Some(TOKEN_NAME.to_string().into_bytes());
            contract.metadata.symbol = Some(TOKEN_SYMBOL.to_string().into_bytes());
            contract.metadata.decimals = TOKEN_DECIMALS;

            // mint with openbrush:
            contract._mint_to(caller, SUPPLY_CAP)
                    .expect("Failed to mint the initial supply");
            contract._init_with_owner(caller);

            // create initial pool balances
            for pool in 0..POOL_COUNT {

                contract.pool.balances[pool] =
                                POOLS[pool].tokens * DECIMALS_POWER10;
            }
            
            contract
        }

////////////////////////////////////////////////////////////////////////////
/////// timing /////////////////////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////

        /// - Function to check if enough time has passed to collect next payout.
        /// - This function ensures Interlock cannot rush the vesting schedule.
        /// - This function must be called before the next round of token distributions.
        #[ink(message)]
        #[openbrush::modifiers(only_owner)]
        pub fn check_time(
            &mut self,
        ) -> PSP22Result<()> {

            // test to see if current time falls beyond time for next payout
            if self.env().block_timestamp() > self.vest.nextpayout {

                // update time variables
                self.vest.nextpayout += ONE_MONTH;
                self.vest.monthspassed += 1;

                return Ok(());
            }

            // too early, do nothing
            return Err(OtherError::PayoutTooEarly.into())
        }
        
        /// - Time in seconds until next payout in minutes.
        #[ink(message)]
        pub fn remaining_time(
            &self
        ) -> Timestamp {

            // calculate remaining time
            let timeleft: Timestamp = match self.vest.nextpayout.checked_sub(self.env().block_timestamp()) {
                Some(difference) => difference,
                None => return 0,
            };

            timeleft
        }

////////////////////////////////////////////////////////////////////////////
/////// stakeholders  //////////////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////

        /// - Function that registers a stakeholder's wallet and vesting info.
        /// - Used to calculate monthly payouts and track net paid.
        /// - Stakeholder data also used for stakeholder to verify their place in vesting schedule.
        #[ink(message)]
        #[openbrush::modifiers(only_owner)]
        pub fn register_stakeholder(
            &mut self,
            stakeholder: AccountId,
            share: Balance,
            pool: u8,
        ) -> PSP22Result<()> {

            // make sure share is > 0
            if share == 0 {
                return Err(OtherError::ShareTooSmall.into());
            }

            // create stakeholder struct
            let this_stakeholder = StakeholderData {
                paid: 0,
                share: share,
                pool: pool,
            };

            // insert stakeholder struct into mapping
            self.vest.stakeholder.insert(stakeholder, &this_stakeholder);

            Ok(())
        }

        /// - Function that returns a stakeholder's payout and other data.
        /// - This will allow stakeholders to verify their stake from explorer if so motivated.
        /// - Returns tuple (StakeholderData, payremaining, payamount, poolnumber).
        #[ink(message)]
        pub fn stakeholder_data(
            &self,
            stakeholder: AccountId,
        ) -> (StakeholderData, Balance, Balance, String) {

            // get pool and stakeholder data structs first
            let this_stakeholder = self.vest.stakeholder.get(stakeholder).unwrap();
            let pool = &POOLS[this_stakeholder.pool as usize];

            // how much has stakeholder already claimed?
            let paidout: Balance = this_stakeholder.paid;

            // how much does stakeholder have yet to collect?
            let payremaining: Balance = this_stakeholder.share - paidout;

            // how much does stakeholder get each month?
            let payamount: Balance = this_stakeholder.share / pool.vests as Balance;

            return (
                this_stakeholder.clone(),
                payremaining,
                payamount,
                POOLS[this_stakeholder.pool as usize].name.to_string(),
            )
        }

////////////////////////////////////////////////////////////////////////////
/////// token distribution /////////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////

        /// - General function to transfer the token share a stakeholder is currently entitled to.
        /// - This is called once per stakeholder by Interlock, Interlock paying fees.
        /// - Pools are guaranteed to have enough tokens for all stakeholders.
        #[ink(message)]
        #[openbrush::modifiers(only_owner)]
        pub fn distribute_tokens(
            &mut self,
            stakeholder: AccountId,
        ) -> PSP22Result<()> {

            // get data structs
            let mut this_stakeholder = match self.vest.stakeholder.get(stakeholder) {
                Some(s) => s,
                None => { return Err(OtherError::StakeholderNotFound.into()) },
            };
            let pool = &POOLS[this_stakeholder.pool as usize];

            // require cliff to have been surpassed
            if self.vest.monthspassed < pool.cliffs as u16 {
                return Err(OtherError::CliffNotPassed.into())
            }

            // require share has not been completely paid out
            if this_stakeholder.paid == this_stakeholder.share {
                return Err(OtherError::StakeholderSharePaid.into())
            }

            // calculate the payout owed
            // ! no checked_div needed; pool.vests guaranteed to be nonzero
            let mut payout: Balance = this_stakeholder.share / pool.vests as Balance;

            // require that payout isn't repeatable for this month
            // ! no checked_div needed; this_stakeholder.share guaranteed to be nonzero
            let payments = this_stakeholder.paid / payout;
            if payments >= self.vest.monthspassed as u128 {
                return Err(OtherError::PayoutTooEarly.into())
            }

            // calculate the new total paid to stakeholder
            let mut newpaidtotal: Balance = match this_stakeholder.paid.checked_add(payout) {
                Some(sum) => sum,
                None => return Err(OtherError::Overflow.into()),
            };

            // calculate remaining share
            let remainingshare: Balance = match this_stakeholder.share.checked_sub(newpaidtotal) {
                Some(difference) => difference,
                None => return Err(OtherError::Underflow.into()),
            };

            // if this is final payment, add token remainder to payout
            // (this is to compensate for floor division that calculates payamount)
            if remainingshare < payout {

                payout += remainingshare;
                newpaidtotal = this_stakeholder.share;
            }

            // now transfer tokens
            let _ = self.transfer(stakeholder, payout, Default::default())?;

            // update pool balance
            match self.pool.balances[this_stakeholder.pool as usize].checked_sub(payout) {
                Some(difference) => self.pool.balances[this_stakeholder.pool as usize] = difference,
                None => return Err(OtherError::Underflow.into()),
            };

            // finally update stakeholder data struct state
            this_stakeholder.paid = newpaidtotal;
            self.vest.stakeholder.insert(stakeholder, &this_stakeholder);

            Ok(())
        }

        /// - Function used to payout tokens to pools with no vesting schedule.
        /// POOL ARGUMENTS:
        ///      PARTNERS
        ///      WHITELIST
        ///      PUBLIC_SALE
        ///      PROCEEDS
        #[ink(message)]
        #[openbrush::modifiers(only_owner)]
        pub fn payout_tokens(
            &mut self,
            stakeholder: AccountId,
            amount: Balance,
            pool: String,
        ) -> PSP22Result<()> {

            let poolnumber: u8 = match pool.as_str() {
                "PARTNERS"      => 9,
                "WHITELIST"     => 10,
                "PUBLIC_SALE"   => 11,
                "PROCEEDS"      => {
                    // deduct payout amount
                    match self.pool.proceeds.checked_sub(amount) {
                        Some(difference) => self.pool.proceeds = difference,
                        None => return Err(OtherError::PaymentTooLarge.into()),
                    };
                    // now transfer tokens
                    let _ = self.transfer(stakeholder, amount, Default::default())?;
                    return Ok(());
                },
                _ => return Err(OtherError::InvalidPool.into())
            };

            // deduct payout amount
            match self.pool.balances[poolnumber as usize].checked_sub(amount) {
                Some(difference) => self.pool.balances[poolnumber as usize] = difference,
                None => return Err(OtherError::PaymentTooLarge.into()),
            };

            // now transfer tokens
            let _ = self.transfer(stakeholder, amount, Default::default())?;

            Ok(())
        }

////////////////////////////////////////////////////////////////////////////
/////// pool data //////////////////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////

        /// - Function that returns pool data.
        /// - This will allow observers to verify vesting parameters for each pool (esp. theirs).
        /// - Observers may verify pool data from explorer if so motivated.
        /// - Pool numbers range from 0-11.
        /// - Returns (name, tokens, vests, cliff) (formatted for convenient for Substrate UI)..
        #[ink(message)]
        pub fn pool_data(
            &self,
            poolnumber: u8,
        ) -> (String, String, String, String) {
        
            let pool = &POOLS[poolnumber as usize];

            return (
                format!("pool: {:?} ", pool.name.to_string()),
                format!("tokens alotted: {:?} ", pool.tokens),
                format!("number of vests: {:?} ", pool.vests),
                format!("vesting cliff: {:?} ", pool.cliffs),
            )
        }
        
        /// - Get current balance of any vesting pool.
        #[ink(message)]
        pub fn pool_balance(
            &self,
            pool: u8,
        ) -> (String, Balance) {

            (format!("pool: {:?}, balance: {:?}", 
                    POOLS[pool as usize].name.to_string(),
                    self.pool.balances[pool as usize]),
             self.pool.balances[pool as usize])
        }

        /// - Display proceeds pool balance.
        #[ink(message)]
        pub fn proceeds_available(
            &self,
        ) -> Balance {

            self.pool.proceeds
        }

////////////////////////////////////////////////////////////////////////////
//// rewarding  ////////////////////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////

        /// - Reward the interlocker for browsing, etc.
        /// - This is a manual rewarding function, to override the socket formalism.
        #[ink(message)]
        #[openbrush::modifiers(only_owner)]
        pub fn reward_interlocker(
            &mut self,
            reward: Balance,
            interlocker: AccountId
        ) -> PSP22Result<Balance> {

            // make sure reward not too large
            if self.pool.balances[REWARDS as usize] < reward {
                return Err(OtherError::PaymentTooLarge.into())
            }

            // update total amount rewarded to interlocker
            match self.reward.total.checked_add(reward) {
                Some(sum) => self.reward.total = sum,
                None => return Err(OtherError::PaymentTooLarge.into()),
            };

            // update rewards pool balance
            // (contract calls transfer, not owner, thus we must update here)
            match self.pool.balances[REWARDS as usize].checked_sub(reward) {
                Some(difference) => self.pool.balances[REWARDS as usize] = difference,
                None => return Err(OtherError::PaymentTooLarge.into()),
            };

            // transfer reward tokens from rewards pool to interlocker
            let _ = self.transfer(interlocker, reward, Default::default())?;

            // get previous total rewarded to interlocker
            let rewardedinterlockertotal: Balance = match self.reward.interlocker.get(interlocker) {
                Some(total) => total,
                None => 0,
            };

            // compute and update new total awarded to interlocker
            let newrewardedtotal: Balance = match rewardedinterlockertotal.checked_add(reward) {
                Some(sum) => sum,
                None => return Err(OtherError::PaymentTooLarge.into()),
            };
            self.reward.interlocker.insert(interlocker, &newrewardedtotal);

            // emit Reward event
            self.env().emit_event(Reward {
                to: Some(interlocker),
                amount: reward,
            });

            // this returns interlocker total reward amount for extension display purposes
            Ok(newrewardedtotal)
        }

        /// - Get amount rewarded to interlocker to date.
        #[ink(message)]
        pub fn rewarded_interlocker_total(
            &self,
            interlocker: AccountId
        ) -> Balance {

            match self.reward.interlocker.get(interlocker) {
                Some(total) => total,
                None => 0,
            }
        }

        /// - Get total amount rewarded to date.
        #[ink(message)]
        pub fn rewarded_total(
            &self
        ) -> Balance {

            self.reward.total
        }

////////////////////////////////////////////////////////////////////////////
//// misc  /////////////////////////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////

        /// - Function to get the number of months passed for contract.
        #[ink(message)]
        pub fn months_passed(
            &self,
        ) -> u16 {

            self.vest.monthspassed
        }

        /// - Function to get the supply cap minted on TGE.
        #[ink(message)]
        pub fn cap(
            &self,
        ) -> Balance {

            SUPPLY_CAP
        }

////////////////////////////////////////////////////////////////////////////
//// portability and extensibility  ////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////

        /// - Modifies the code which is used to execute calls to this contract address.
        /// - This upgrades the token contract logic while using old state.
        #[ink(message)]
        #[openbrush::modifiers(only_owner)]
        pub fn update_contract(
            &mut self,
            code_hash: [u8; 32]
        ) -> PSP22Result<()> {

            // takes code hash of updates contract and modifies preexisting logic to match
            ink::env::set_code_hash(&code_hash).unwrap_or_else(|err| {
                panic!(
                    "Failed to `set_code_hash` to {:?} due to {:?}",
                    code_hash, err
                )
            });

            Ok(())
        }

        /// - Create a new port that application contract can register with.
        /// - Each port tracks amount rewarded, tax collected, if it is locked or not, owner.
        /// - A locked port may only be registered by the Interlock Network foundation.
        #[ink(message)]
        #[openbrush::modifiers(only_owner)]
        pub fn create_port(
            &mut self,
            codehash: Hash,
            tax: Balance,
            cap: Balance,
            locked: bool,
            number: u16,
            owner: AccountId,
        ) -> PSP22Result<()> {

            let port = Port {
                application: codehash,     // <--! a port defines an external staking/reward contract plus any
                tax: tax,                  //      custom logic preceding the tax_and_reward() function
                cap: cap,
                locked: locked,
                paid: 0,
                collected: 0,
                owner: owner,
            };
            self.app.ports.insert(number, &port);

            Ok(())
        }

        /// - Rewards/staking/application contracts register with this token contract here.
        /// - Contract must first register with token contract as port to allow connection via
        /// socket.
        #[ink(message)]
        pub fn create_socket(
            &mut self,
            operator: AccountId,
            portnumber: u16,
        ) -> OtherResult<()> {

            // get application address
            let application: AccountId = self.env().caller();

            // make sure caller is a contact, return if not
            if !self.env().is_contract(&application) {
                return Err(OtherError::NotContract);
            };

            // get hash of calling contract
            let callinghash: Hash = match self.env().code_hash(&application) {
                Ok(hash) => hash,
                Err(_) => return Err(OtherError::NotContract),
            };

            // get port specified by calling contract
            let port: Port = match self.app.ports.get(portnumber) {
                Some(port) => port,
                None => return Err(OtherError::NoPort),
            };

            // make sure port is unlocked, or caller is token contract owner (interlock)
            //   . this makes it so that people can't build their own client application
            //     to 'hijack' an approved and registered rewards contract.
            //   . if port is locked then only interlock can create new socket with port
            //   . socket creation is only called by an external contract that the port represents
            if port.locked && (self.ownable.owner != operator) {
                return Err(OtherError::PortLocked);
            }
            
            // compare calling contract hash to registered port hash
            // to make sure it is safe (ie, approved and audited by interlock)
            if callinghash == port.application {
                
                // if the same, contract is allowed to create socket (socket == operatoraddress:portnumber)
                let socket = Socket { operator: operator, portnumber: portnumber };

                // socket is registered with token contract thus the calling
                // contract that created the socket may start calling socket to receive rewards
                self.app.sockets.insert(application, &socket);
            
                // setup socket according to port type
                match portnumber {

                    // Interlock-owned UANFTs
                    0 => { /* do nothing */ },

                    // non-Interlock-owned UANFTs
                    1 => { /* do nothing */ },

                    // Interlock gray area staking applications
                    2 => {

                        // give socket allowance up to port cap
                        //   . connecting contracts will not be able to reward
                        //     more than cap specified by interlock (this may be a stipend, for example)
                        //   . rewards fail to transfer if the amount paid plus the reward exceeds cap
                        self.psp22.allowances.insert(
                            &(&self.ownable.owner, &application),
                            &port.cap
                        );

                        self._emit_approval_event(self.ownable.owner, application, port.cap);
                    },
                    _ => return Err(OtherError::InvalidPort),

                };

                return Ok(()) 
            }

            // returns error if calling staking application contract is not a known
            // safe contract registered by interlock as a 'port' 
            Err(OtherError::UnsafeContract)
        }

        /// - Check for socket and apply custom logic after being called from application contract.
        #[ink(message)]
        pub fn call_socket(
            &mut self,
            address: AccountId,
            amount: Balance,
            _data: Vec<u8>,
        ) -> OtherResult<()> {

            // make sure address is not contract; we do not want to reward contracts
            if self.env().is_contract(&address) {
                return Err(OtherError::CannotRewardContract);
            }

            // get socket, to get port assiciated with socket
            let socket: Socket = match self.app.sockets.get(self.env().caller()) {
                Some(socket) => socket,
                None => return Err(OtherError::NoSocket),
            };

            // get port info
            let mut port: Port = match self.app.ports.get(socket.portnumber) {
                Some(port) => port,
                None => return Err(OtherError::NoPort),
            };

            // apply custom logic for given port
            match socket.portnumber {

                // NOTE: injecting custom logic into port requires Interlock Token
                //       contract codehash update after internal port contract audit
                
                // PORT 0 == Interlock-owned UANFTs
                //
                // This socket call is a UANFT self-mint operation with ILOCK proceeds returning to
                // rewards pool
                0 => { 

                    // deduct cost of uanft from minter's account
                    let mut minterbalance: Balance = self.psp22.balance_of(address);
                    match minterbalance.checked_sub(amount) {
                        Some(difference) => minterbalance = difference,
                        None => return Err(OtherError::Underflow),
                    };
                    self.psp22.balances.insert(&address, &minterbalance);
                
                    // update pools
                    match self.pool.balances[REWARDS as usize].checked_add(amount) {
                        Some(sum) => self.pool.balances[REWARDS as usize] = sum,
                        None => return Err(OtherError::Overflow),
                    };
                    match self.pool.circulating.checked_sub(amount) {
                        Some(difference) => self.pool.circulating = difference,
                        None => return Err(OtherError::Underflow),
                    };

                    // update port
                    match port.paid.checked_add(amount) {
                        Some(sum) => port.paid = sum,
                        None => return Err(OtherError::Overflow),
                    };
                    self.app.ports.insert(socket.portnumber, &port);
                },

                // PORT 1 == Non-Interlock-owned UANFTs
                //
                // This socket call is for a UANFT self-mint operation that is taxed by Interlock
                // but mint ILOCK proceeds go to socket operator instead of Interlock
                1 => {

                    // deduct cost of uanft from minter's account
                    let mut minterbalance: Balance = self.psp22.balance_of(address);
                    match minterbalance.checked_sub(amount) {
                        Some(difference) => minterbalance = difference,
                        None => return Err(OtherError::Underflow),
                    };
                    self.psp22.balances.insert(&address, &minterbalance);

                    let adjustedamount: Balance = self.tax_port_transfer(socket, port, amount)?;

                    // increment cost of uanft to operator's account
                    let mut operatorbalance: Balance = self.psp22.balance_of(socket.operator);
                    match operatorbalance.checked_add(adjustedamount) {
                        Some(sum) => operatorbalance = sum,
                        None => return Err(OtherError::Overflow),
                    };
                    self.psp22.balances.insert(&socket.operator, &operatorbalance);
                    
                    // emit Transfer event, uanft transfer
                    self.env().emit_event(Transfer {
                        from: Some(address),
                        to: Some(socket.operator),
                        amount: adjustedamount,
                    });

                },

                // PORT 2 == reserved for Interlock gray-area staking applications
                //
                // reserved Interlock ports
                2 => { /* gray area staking rewards logic here */ },

                // .
                // .
                // .
                //

                // ... custom logic example:
                65535 => {

                    // < inject custom logic here BEFORE tax_and_reward >
                    // <    (ie, do stuff with port and socket data)    >
                },

                _ => return Err(OtherError::InvalidPort),
            };

            Ok(())
        }

        /// - Tax and reward transfer between socket calling address and socket operator.
        pub fn tax_port_transfer(
            &mut self,
            socket: Socket,
            mut port: Port,
            amount: Balance,
        ) -> OtherResult<Balance> {

            // compute tax - tax number is in centipercent, 0.01% ==> 100% = 1 & 0.01% = 10_000
            //
            // a tax of 0.01% is amount/10_000
            let tax: Balance = match amount.checked_div(port.tax as Balance) {
                Some(quotient) => quotient,
                None => return Err(OtherError::DivError),
            };

            // update proceeds pool and total circulation
            match self.pool.proceeds.checked_add(tax) {
                Some(sum) => self.pool.proceeds = sum,
                None => return Err(OtherError::Overflow),
            };
            match self.pool.circulating.checked_sub(tax) {
                Some(difference) => self.pool.circulating = difference,
                None => return Err(OtherError::Underflow),
            };

            // update port (paid and collected) 
            match port.collected.checked_add(tax) {
                Some(sum) => port.collected = sum,
                None => return Err(OtherError::Overflow),
            };
            let adjustedamount: Balance = match amount.checked_sub(tax) {
                Some(difference) => difference,
                None => return Err(OtherError::Underflow),
            };
            match port.paid.checked_add(adjustedamount) {
                Some(sum) => port.paid = sum,
                None => return Err(OtherError::Overflow),
            };
            self.app.ports.insert(socket.portnumber, &port);
                    
            // emit Transfer event, operator to ILOCK proceeds pool
            self.env().emit_event(Transfer {
                from: Some(socket.operator), // we do not tax port owner,
                to: Some(self.ownable.owner),// rather we tax xfer itself in this case
                amount: tax,
            });

            // return adjusted amount
            Ok(amount - tax)
        }

        /// - Get socket info.
        #[ink(message)]
        pub fn socket(
            &self,
            application: AccountId,
        ) -> Socket {
            
            match self.app.sockets.get(application) {
                Some(socket) => socket,
                None => Default::default(),
            }
        }

        /// - Get port info.
        #[ink(message)]
        pub fn port(
            &self,
            portnumber: u16,
        ) -> Port {
            
            match self.app.ports.get(portnumber) {
                Some(port) => port,
                None => Default::default(),
            }
        }        
    

////////////////////////////////////////////////////////////////////////////
//// testing helpers ///////////////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////



        /// - Function to increment monthspassed for testing.
        ///
        ///     MUST BE DELETED PRIOR TO TGE
        ///
        #[ink(message)]
        #[openbrush::modifiers(only_owner)]
        pub fn TESTING_increment_month(
            &mut self,
        ) -> OtherResult<bool> {

            self.vest.monthspassed += 1;

            Ok(true)
        }
    } // END OF ILOCKmvp IMPL BLOCK

//
// TESTING INCOMPLETE
//
// . To view debug prints and assertion failures run test via:
//
//      cargo +nightly test --features e2e-tests -- --show-output
//
// . To view debug for specific method run test via:
//
//      cargo +nightly test <test_function_here> -- --nocapture
//
// . To run end-to-end tests, first make sure you have the substrate
//   dev node capabilities installed via:
//
//      cargo install contracts-node --git https://github.com/paritytech/substrate-contracts-node.git
//
//   Then run the node:
//
//      substrate-contracts-node
//


// TEST TODO
// in order of appearance
//
// [x] happyunit_total_supply                <-- checked within new_token()
// [x] happye2e_transfer             \
// [] sade2e_transfer                |
// [x] happye2e_transfer_from        |---- we test these because we change the default openbrush
// [] sade2e_transfer_from           |     implementations ... per agreement with Kudelski, we will
// [x] happye2e_burn                 |     be assuming that openbrush is safe ... we may wish to perform
// [] sade2e_burn                    /     additional tests once audit is underway or/ in general future
// [x] happyunit_new_token (no sad, returns only Self)
// [!] happyunit_check_time                  <-- not possible to advance block, TEST ON TESTNET
// [!] sadunit_check_time                    <-- not possible to advance block, TEST ON TESTNET
// [!] happyunit_remaining_time              <-- not possible to advance block, TEST ON TESTNET
// [x] happyunit_register_stakeholder        <-- this checked within distribute_tokens()
// [] sadunit_register_stakeholder ... add sad case where share is greater than pool total?
// [x] happyunit_stakeholder_data            <-- checked within distriut_tokens()
// [x] happye2e_distribute_tokens            <-- this is to check that the vesting schedule works...
// [x] happye2e_payout_tokens                 ...month passage is artificial here, without 
// [] sade2e_payout_tokens                    advancing blocks.
// [x] happyunit_pool_data
// [x] happye2e_reward_interlocker           
// [x] happyunit_rewarded_interlocker_total  <-- checked within reward_interlocker()
// [x] happyunit_rewarded_total              <-- checked within reward_interlocker() 
// [x] happyunit_months_passed               <-- checked within new_token()
// [x] happyunit_cap                         <-- checked within new_token()
// [!] happyunit_update_contract             <-- TEST ON TESTNET
// [] sadunit_update_contract
// [x] happyunit_create_port
//      [x] happyunit_port                   <-- checked within create_port()
// [] ** happye2e_create_socket     \
// [] ** sade2e_create_socket       |----- these must be performed from generic port
// [] ** happye2e_call_socket       |      or from the uanft contract's self minting message
// [] ** sade2e_call_socket         /
// [x] happyunit_tax_port_transfer
// [] sadunit_tax_port_transfer
// [x] happyunit_check_time
//

// * note ... unit and end to end tests must reside in separate modules
//
// * note ... PSP22 token standart errors as implemented by openbrush:
//
//    /// Returned if not enough balance to fulfill a request is available.
//    InsufficientBalance,
//    /// Returned if not enough allowance to fulfill a request is available.
//    InsufficientAllowance,
//    /// Returned if recipient's address is zero.
//    ZeroRecipientAddress,
//    /// Returned if sender's address is zero.
//    ZeroSenderAddress,

////////////////////////////////////////////////////////////////////////////
//// end to end ////////////////////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////
    
    #[cfg(all(test, feature = "e2e-tests"))]
    mod e2e_tests {

        use super::*;
        use ink_e2e::{
            build_message,
        };
        use openbrush::contracts::psp22::{
            psp22_external::PSP22,
            extensions::burnable::psp22burnable_external::PSP22Burnable,
        };

        type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

        /// HAPPY TRANSFER
        /// - Test if customized transfer function works correctly.
        /// - When transfer from contract owner, circulating supply increases.
        /// - When transfer to contract owner, circulating supply decreases
        /// and rewards pool increases.
        #[ink_e2e::test]
        async fn happye2e_transfer(
            mut client: ink_e2e::Client<C, E>,
        ) -> E2EResult<()> {

            let alice_account = ink_e2e::account_id(ink_e2e::AccountKeyring::Alice);
            let bob_account = ink_e2e::account_id(ink_e2e::AccountKeyring::Bob);

            let constructor = ILOCKmvpRef::new_token();
            let contract_acct_id = client
                .instantiate("ilockmvp", &ink_e2e::alice(), constructor, 0, None)
                .await.expect("instantiate failed").account_id;

            // alice is contract owner
            // transfers 1000 ILOCK from alice to bob and check for resulting Transfer event
            let alice_transfer_msg = build_message::<ILOCKmvpRef>(contract_acct_id.clone())
                .call(|contract| contract.transfer(bob_account.clone(), 1000, Vec::new()));
            let transfer_response = client
                .call(&ink_e2e::alice(), alice_transfer_msg, 0, None).await.unwrap();
            
            // filter for transfer event
            let contract_emitted_transfer = transfer_response
                .events
                .iter()
                .find(|event| {
                    event
                        .as_ref()
                        .expect("expected event")
                        .event_metadata()
                        .event()
                        == "ContractEmitted" &&
                        String::from_utf8_lossy(
                            event.as_ref().expect("bad event").bytes()).to_string()
                       .contains("ILOCKmvp::Transfer")
                })
                .expect("Expect ContractEmitted event")
                .unwrap();

            // Decode to the expected event type (skip field_context)
            let transfer_event = contract_emitted_transfer.field_bytes();
            let decoded_transfer =
                <Transfer as scale::Decode>::decode(&mut &transfer_event[35..]).expect("invalid data");

            // Destructor decoded event
            let Transfer { from, to, amount } = decoded_transfer;

            // Assert with the expected value
            assert_eq!(from, Some(alice_account), "encountered invalid Transfer.from");
            assert_eq!(to, Some(bob_account), "encountered invalid Transfer.to");
            assert_eq!(amount, 1000, "encountered invalid Transfer.amount");    

            // checks that bob has expected resulting balance
            let bob_balance_msg = build_message::<ILOCKmvpRef>(contract_acct_id.clone())
                .call(|contract| contract.balance_of(bob_account.clone()));
            let bob_balance = client
                .call_dry_run(&ink_e2e::bob(), &bob_balance_msg, 0, None).await.return_value();
            assert_eq!(0 + 1000, bob_balance);

            // checks that alice has expected resulting balance
            let alice_balance_msg = build_message::<ILOCKmvpRef>(contract_acct_id.clone())
                .call(|contract| contract.balance_of(alice_account.clone()));
            let alice_balance = client
                .call_dry_run(&ink_e2e::alice(), &alice_balance_msg, 0, None).await.return_value();
            assert_eq!(SUPPLY_CAP - 1000, alice_balance);

            // checks that circulating supply increased appropriately
            let total_supply_msg = build_message::<ILOCKmvpRef>(contract_acct_id.clone())
                .call(|contract| contract.total_supply());
            let mut total_supply = client
                .call_dry_run(&ink_e2e::alice(), &total_supply_msg, 0, None).await.return_value();
            assert_eq!(0 + 1000, total_supply);

            // transfers 500 ILOCK from bob to alice
            let bob_transfer_msg = build_message::<ILOCKmvpRef>(contract_acct_id.clone())
                .call(|contract| contract.transfer(alice_account.clone(), 500, Vec::new()));
            let _result = client
                .call(&ink_e2e::bob(), bob_transfer_msg, 0, None).await;
               
            // checks that circulating supply decreased appropriately
            total_supply = client
                .call_dry_run(&ink_e2e::alice(), &total_supply_msg, 0, None).await.return_value();
            assert_eq!(1000 - 500, total_supply);

            // check that rewards supply increased appropriately
            let rewards_balance_msg = build_message::<ILOCKmvpRef>(contract_acct_id.clone())
                .call(|contract| contract.pool_balance(REWARDS));
            let rewards_balance = client
                .call_dry_run(&ink_e2e::alice(), &rewards_balance_msg, 0, None).await.return_value().1;
            assert_eq!(POOLS[REWARDS as usize].tokens * DECIMALS_POWER10 + 500, rewards_balance);

            Ok(())
        }

        /// SAD TRANSFER
        /// - Test if customized transfer function fails correctly.
        ///
        /// - Return
        ///     InsufficientBalance     - When caller has allowance < amount
        ///     ZeroRecipientAddress    - when to's address is AccountId::from([0_u8; 32])
        ///     ZeroSenderAddress       - When caller's address is AccountId::from([0_u8; 32])
        ///                               (zero address has known private key..however that works)
        #[ink_e2e::test]
        async fn sade2e_transfer(
            mut client: ink_e2e::Client<C, E>,
        ) -> E2EResult<()> {

            Ok(())
        }

        /// HAPPY TRANSFER_FROM
        /// - Test if customized transfer_from function works correctly.
        /// - When transfer from contract owner, circulating supply increases.
        /// - Transfer and Approval events are emitted.
        /// - When transfer to contract owner, circulating supply decreases
        /// - When caller transfers, their allowace with from decreases
        ///   and rewards pool increases
        #[ink_e2e::test]
        async fn happye2e_transfer_from(
            mut client: ink_e2e::Client<C, E>,
        ) -> E2EResult<()> {

            let alice_account = ink_e2e::account_id(ink_e2e::AccountKeyring::Alice);
            let bob_account = ink_e2e::account_id(ink_e2e::AccountKeyring::Bob);
            let charlie_account = ink_e2e::account_id(ink_e2e::AccountKeyring::Charlie);

            let constructor = ILOCKmvpRef::new_token();
            let contract_acct_id = client
                .instantiate("ilockmvp", &ink_e2e::alice(), constructor, 0, None)
                .await.expect("instantiate failed").account_id;

            // alice approves bob 1000 ILOCK
            let alice_approve_msg = build_message::<ILOCKmvpRef>(contract_acct_id.clone())
                .call(|contract| contract.approve(bob_account.clone(), 1000));
            let _approval_result = client
                .call(&ink_e2e::alice(), alice_approve_msg, 0, None).await;
            
            // bob transfers 1000 ILOCK from alice to charlie
            let bob_transfer_from_msg = build_message::<ILOCKmvpRef>(contract_acct_id.clone())
                .call(|contract| contract.transfer_from(
                    alice_account.clone(), charlie_account.clone(), 1000, Vec::new())
            );
            let transfer_from_response = client
                .call(&ink_e2e::bob(), bob_transfer_from_msg, 0, None).await.unwrap();
            
            // filter for approval event
            let contract_emitted_approval = transfer_from_response
                .events
                .iter()
                .find(|event| {
                    event
                        .as_ref()
                        .expect("expected event")
                        .event_metadata()
                        .event()
                        == "ContractEmitted" &&
                        String::from_utf8_lossy(
                            event.as_ref().expect("bad event").bytes()).to_string()
                       .contains("ILOCKmvp::Approval")
                })
                .expect("Expect ContractEmitted event")
                .unwrap();

            // decode to the expected event type (skip field_context)
            let approval_event = contract_emitted_approval.field_bytes();
            let decoded_approval =
                <Approval as scale::Decode>::decode(&mut &approval_event[35..]).expect("invalid data");

            // destructor decoded eapproval
            let Approval { owner, spender, amount } = decoded_approval;

            // assert with the expected value
            assert_eq!(owner, Some(alice_account), "encountered invalid Approval.owner");
            assert_eq!(spender, Some(bob_account), "encountered invalid Approval.spender");
            assert_eq!(amount, 1000 - 1000, "encountered invalid Approval.amount");  
            
            // filter for transfer event
            let contract_emitted_transfer = transfer_from_response
                .events
                .iter()
                .find(|event| {
                    event
                        .as_ref()
                        .expect("expected event")
                        .event_metadata()
                        .event()
                        == "ContractEmitted" &&
                        String::from_utf8_lossy(
                            event.as_ref().expect("bad event").bytes()).to_string()
                       .contains("ILOCKmvp::Transfer")
                })
                .expect("Expect ContractEmitted event")
                .unwrap();

            // decode to the expected event type (skip field_context)
            let transfer_event = contract_emitted_transfer.field_bytes();
            let decoded_transfer =
                <Transfer as scale::Decode>::decode(&mut &transfer_event[35..]).expect("invalid data");

            // destructor decoded transfer
            let Transfer { from, to, amount } = decoded_transfer;

            // assert with the expected value
            assert_eq!(from, Some(alice_account), "encountered invalid Transfer.from");
            assert_eq!(to, Some(charlie_account), "encountered invalid Transfer.to");
            assert_eq!(amount, 1000, "encountered invalid Transfer.amount");  
            
            // checks that charlie has expected resulting balance
            let charlie_balance_msg = build_message::<ILOCKmvpRef>(contract_acct_id.clone())
                .call(|contract| contract.balance_of(charlie_account.clone()));
            let charlie_balance = client
                .call_dry_run(&ink_e2e::charlie(), &charlie_balance_msg, 0, None).await.return_value();
            assert_eq!(0 + 1000, charlie_balance);

            // checks that circulating supply increased appropriately
            let total_supply_msg = build_message::<ILOCKmvpRef>(contract_acct_id.clone())
                .call(|contract| contract.total_supply());
            let mut total_supply = client
                .call_dry_run(&ink_e2e::alice(), &total_supply_msg, 0, None).await.return_value();
            assert_eq!(0 + 1000, total_supply);

            // checks that bob's allowance decreased appropriately
            let bob_allowance_msg = build_message::<ILOCKmvpRef>(contract_acct_id.clone())
                .call(|contract| contract.allowance(alice_account.clone(), bob_account.clone()));
            let bob_allowance = client
                .call_dry_run(&ink_e2e::alice(), &bob_allowance_msg, 0, None).await.return_value();
            assert_eq!(1000 - 1000, bob_allowance);

            // charlie approves bob 1000 ILOCK
            let charlie_approve_msg = build_message::<ILOCKmvpRef>(contract_acct_id.clone())
                .call(|contract| contract.approve(bob_account.clone(), 1000));
            let _approval_result = client
                .call(&ink_e2e::charlie(), charlie_approve_msg, 0, None).await;

            // bob transfers 1000 ILOCK from charlie to alice
            let bob_transfer_from_msg = build_message::<ILOCKmvpRef>(contract_acct_id.clone())
                .call(|contract| contract.transfer_from(
                    charlie_account.clone(), alice_account.clone(), 1000, Vec::new()));
            let _transfer_from_result = client
                .call(&ink_e2e::bob(), bob_transfer_from_msg, 0, None).await;

            // checks that circulating supply decreased appropriately
            total_supply = client
                .call_dry_run(&ink_e2e::alice(), &total_supply_msg, 0, None).await.return_value();
            assert_eq!(1000 - 1000, total_supply);

            // check that rewards supply increased appropriately
            let rewards_balance_msg = build_message::<ILOCKmvpRef>(contract_acct_id.clone())
                .call(|contract| contract.pool_balance(REWARDS));
            let rewards_balance = client
                .call_dry_run(&ink_e2e::alice(), &rewards_balance_msg, 0, None).await.return_value().1;
            assert_eq!(POOLS[REWARDS as usize].tokens * DECIMALS_POWER10 + 1000, rewards_balance);

            Ok(())
        }

        /// SAD TRANSFER_FROM
        /// - Test if customized transfer_from function fails correctly.
        ///
        /// - Return
        ///     InsufficientBalance     - When caller has allowance < amount
        ///     InsufficientAllowance   - When caller specs amount > from's balance
        ///     ZeroRecipientAddress    - when to's address is AccountId::from([0_u8; 32])
        ///     ZeroSenderAddress       - When from's address is AccountId::from([0_u8; 32])
        #[ink_e2e::test]
        async fn sade2e_transfer_from(
            mut client: ink_e2e::Client<C, E>,
        ) -> E2EResult<()> {

            Ok(())
        }

        /// HAPPY BURN
        /// - Test if customized burn function works correctly.
        /// - When burn occurs, donor balance decreases.
        /// - When burn occurs, circulating supply (total_supply()) decreases
        #[ink_e2e::test]
        async fn happye2e_burn(
            mut client: ink_e2e::Client<C, E>,
        ) -> E2EResult<()> {

            let alice_account = ink_e2e::account_id(ink_e2e::AccountKeyring::Alice);
            let bob_account = ink_e2e::account_id(ink_e2e::AccountKeyring::Bob);

            let constructor = ILOCKmvpRef::new_token();
            let contract_acct_id = client
                .instantiate("ilockmvp", &ink_e2e::alice(), constructor, 0, None)
                .await.expect("instantiate failed").account_id;

            // alice transfers 1000 ILOCK to bob (to check !owner burn)
            let alice_transfer_msg = build_message::<ILOCKmvpRef>(contract_acct_id.clone())
                .call(|contract| contract.transfer(
                    bob_account.clone(), 1000, Vec::new()));
            let _transfer_result = client
                .call(&ink_e2e::alice(), alice_transfer_msg, 0, None).await;

            // alice burns 1000 tokens
            let alice_burn_msg = build_message::<ILOCKmvpRef>(contract_acct_id.clone())
                .call(|contract| contract.burn(alice_account.clone(), 1000));
            let burn_response = client
                .call(&ink_e2e::alice(), alice_burn_msg, 0, None).await.unwrap();

           
            let contract_emitted_transfer = burn_response
                .events
                .iter()
                .find(|event| {
                    event
                        .as_ref()
                        .expect("expected event")
                        .event_metadata()
                        .event()
                        == "ContractEmitted" &&
                        String::from_utf8_lossy(
                            event.as_ref().expect("bad event").bytes()).to_string()
                       .contains("ILOCKmvp::Transfer")
                })
                .expect("Expect ContractEmitted event")
                .unwrap();

            // decode to the expected event type (skip field_context)
            let transfer_event = contract_emitted_transfer.field_bytes();
            let decoded_transfer =
                <Transfer as scale::Decode>::decode(&mut &transfer_event[34..]).expect("invalid data");

            // Destructor decoded event
            let Transfer { from, to, amount } = decoded_transfer;

            // Assert with the expected value
            assert_eq!(from, Some(alice_account), "encountered invalid Transfer.fromr");
            assert_eq!(to, None, "encountered invalid Transfer.to");
            assert_eq!(amount, 1000, "encountered invalid Transfer.amount");  
            
            // checks that alice has expected resulting balance
            let alice_balance_msg = build_message::<ILOCKmvpRef>(contract_acct_id.clone())
                .call(|contract| contract.balance_of(alice_account.clone()));
            let alice_balance = client
                .call_dry_run(&ink_e2e::alice(), &alice_balance_msg, 0, None).await.return_value();
            assert_eq!(SUPPLY_CAP - 1000 - 1000, alice_balance);

            // checks that reward pool decreased appropriately
            let rewards_balance_msg = build_message::<ILOCKmvpRef>(contract_acct_id.clone())
                .call(|contract| contract.pool_balance(REWARDS));
            let rewards_balance = client
                .call_dry_run(&ink_e2e::alice(), &rewards_balance_msg, 0, None).await.return_value().1;
            assert_eq!(POOLS[REWARDS as usize].tokens * DECIMALS_POWER10 - 1000, rewards_balance);

            // bob burns 500 tokens
            let bob_burn_msg = build_message::<ILOCKmvpRef>(contract_acct_id.clone())
                .call(|contract| contract.burn(bob_account.clone(), 500));
            let _bob_burn_result = client
                .call(&ink_e2e::alice(), bob_burn_msg, 0, None).await;

            // checks that circulating supply decreased appropriately
            let total_supply_msg = build_message::<ILOCKmvpRef>(contract_acct_id.clone())
                .call(|contract| contract.total_supply());
            let total_supply = client
                .call_dry_run(&ink_e2e::alice(), &total_supply_msg, 0, None).await.return_value();
            assert_eq!(1000 - 500, total_supply);

            // checks that bob has expected resulting balance
            let bob_balance_msg = build_message::<ILOCKmvpRef>(contract_acct_id.clone())
                .call(|contract| contract.balance_of(bob_account.clone()));
            let bob_balance = client
                .call_dry_run(&ink_e2e::charlie(), &bob_balance_msg, 0, None).await.return_value();
            assert_eq!(1000 - 500, bob_balance);

            Ok(())
        }

        /// SAD BURN
        /// - Test if customized transfer_from function fails correctly.
        ///
        /// - Return
        ///     CallerNotOwner          - When caller does not own contract
        ///     InsufficientBalance     - When donor's balance < burn amount
        #[ink_e2e::test]
        async fn sade2e_burn(
            mut client: ink_e2e::Client<C, E>,
        ) -> E2EResult<()> {

            Ok(())
        }
       
        /// HAPPY DISTRIBUTE_TOKENS
        /// - Test if token distribution works as intended per vesting schedule.
        /// - Cycle through entire vesting period.
        /// - Includes optional print table for inspection
        /// - Includes register_stakeholder().
        /// - Includes distribute_tokens().
        #[ink_e2e::test]
        async fn happye2e_distribute_tokens(
            mut client: ink_e2e::Client<C, E>,
        ) -> E2EResult<()> {

            // fire up contract
            let constructor = ILOCKmvpRef::new_token();
            let contract_acct_id = client
                .instantiate("ilockmvp", &ink_e2e::alice(), constructor, 0, None)
                .await.expect("instantiate failed").account_id;

            // register accounts
            let alice_account = ink_e2e::account_id(ink_e2e::AccountKeyring::Alice);
            let stakeholder_account = ink_e2e::account_id(ink_e2e::AccountKeyring::Bob);
            let stakeholder_share = 1_000_000_000;
            let pool_size = POOLS[TEAM_FOUNDERS as usize].tokens * DECIMALS_POWER10;

            // register stakeholder
            let register_stakeholder_msg = build_message::<ILOCKmvpRef>(contract_acct_id.clone())
                .call(|contract| contract.register_stakeholder(
                    stakeholder_account.clone(), stakeholder_share, TEAM_FOUNDERS));
            let _register_stakeholder_result = client
                .call(&ink_e2e::alice(), register_stakeholder_msg, 0, None).await;

            let cliff = POOLS[TEAM_FOUNDERS as usize].cliffs;
            let vests = POOLS[TEAM_FOUNDERS as usize].vests;
            let schedule_end = vests + cliff - 1;
            let schedule_period = vests;
            let payout = 1_000_000_000 / vests as Balance; // 27_777_777
            let last_payout = payout + 1_000_000_000 % vests as Balance; // 27_777_805

            // check stakeholder_data()
            let stakeholder_data_msg = build_message::<ILOCKmvpRef>(contract_acct_id.clone())
                .call(|contract| contract.stakeholder_data(stakeholder_account.clone()));
            let stakeholder_data = client
                .call_dry_run(&ink_e2e::alice(), &stakeholder_data_msg, 0, None).await.return_value();
            assert_eq!(stakeholder_data.0.share, stakeholder_share);
            assert_eq!(stakeholder_data.1, stakeholder_data.0.share);
            assert_eq!(stakeholder_data.2, payout);
            assert_eq!(stakeholder_data.3, "team+founders".to_string());

            // iterate through one vesting schedule
            for month in 0..(schedule_end + 2) {

                if month >= cliff && month <= schedule_end {

                    let distribute_tokens_msg = build_message::<ILOCKmvpRef>(contract_acct_id.clone())
                        .call(|contract| contract.distribute_tokens(stakeholder_account.clone()));
                    let _distribute_tokens_result = client
                        .call(&ink_e2e::alice(), distribute_tokens_msg, 0, None).await;
                }

                let stakeholder_data_msg = build_message::<ILOCKmvpRef>(contract_acct_id.clone())
                    .call(|contract| contract.stakeholder_data(stakeholder_account.clone()));
                let stakeholder_paid = client
                    .call_dry_run(&ink_e2e::alice(), &stakeholder_data_msg, 0, None)
                    .await.return_value().0.paid;

                let stakeholder_balance_msg = build_message::<ILOCKmvpRef>(contract_acct_id.clone())
                    .call(|contract| contract.balance_of(stakeholder_account.clone()));
                let stakeholder_balance = client
                    .call_dry_run(&ink_e2e::alice(), &stakeholder_balance_msg.clone(), 0, None)
                    .await.return_value();

                let pool_balance_msg = build_message::<ILOCKmvpRef>(contract_acct_id.clone())
                    .call(|contract| contract.pool_balance(TEAM_FOUNDERS));
                let pool_balance = client
                    .call_dry_run(&ink_e2e::alice(), &pool_balance_msg.clone(), 0, None)
                    .await.return_value().1;

                let owner_balance_msg = build_message::<ILOCKmvpRef>(contract_acct_id.clone())
                    .call(|contract| contract.balance_of(alice_account.clone()));
                let owner_balance = client
                    .call_dry_run(&ink_e2e::alice(), &owner_balance_msg.clone(), 0, None)
                    .await.return_value();

                let increment_month_msg = build_message::<ILOCKmvpRef>(contract_acct_id.clone())
                    .call(|contract| contract.TESTING_increment_month());
                let _increment_month_result = client
                    .call(&ink_e2e::alice(), increment_month_msg, 0, None).await;

                /* // visual proof of workee
                println!("{:?}", month_result);
                println!("{:?}", stakeholder_paid);
                println!("{:?}", stakeholder_balance);
                println!("{:?}", pool_balance);
                println!("{:?}", owner_balance);
                */
                if month < cliff {

                    assert_eq!(stakeholder_paid, 0);
                    assert_eq!(stakeholder_balance, 0);
                    assert_eq!(owner_balance, SUPPLY_CAP);
                    assert_eq!(pool_balance, pool_size);

                } else if month >= cliff && month < schedule_end {

                    assert_eq!(stakeholder_paid, (month - cliff + 1) as Balance * payout);
                    assert_eq!(stakeholder_balance, (month - cliff + 1) as Balance * payout);
                    assert_eq!(owner_balance, SUPPLY_CAP - (month - cliff + 1) as Balance * payout);
                    assert_eq!(pool_balance, pool_size - (month - cliff + 1) as Balance * payout);

                } else if month >= schedule_end {

                    assert_eq!(stakeholder_paid, (schedule_period - 1) as Balance * payout + last_payout);
                    assert_eq!(stakeholder_balance, (schedule_period - 1) as Balance * payout + last_payout);
                    assert_eq!(owner_balance,
                               SUPPLY_CAP - (schedule_period - 1) as Balance * payout - last_payout);
                    assert_eq!(pool_balance,
                               pool_size - (schedule_period - 1) as Balance * payout - last_payout);
                }
            }
            Ok(())
        }

        /// SAD DISTRIBUTE_TOKENS
        /// - Check to make sure distribute_tokens fails as expected.
        ///
        /// - Return
        ///     CallerNotOwner               - When caller does not own contract
        ///     StakeholderNotFound          - when stakeholder specified isn't registered
        ///     CliffNotPassed               - when pool's vesting cliff isn't passed
        ///     StakeholderSharePaid         - when stakeholder has already been paid entire share
        ///     PayoutTooEarly               - when next month's payment isn't ready
        #[ink_e2e::test]
        async fn sade2e_distribute_tokens(
            mut client: ink_e2e::Client<C, E>,
        ) -> E2EResult<()> {

            Ok(())
        }

        /// HAPPY PAYOUT_TOKENS
        /// - Check to make sure payout_tokens works as expected.
        /// - Checks PARTNERS, WHITELIST, and PUBLIC_SALE pools.
        /// - Checks resulting balances for three pools and recipients.
        #[ink_e2e::test]
        async fn happye2e_payout_tokens(
            mut client: ink_e2e::Client<C, E>,
        ) -> E2EResult<()> {

            let alice_account = ink_e2e::account_id(ink_e2e::AccountKeyring::Alice);
            let bob_account = ink_e2e::account_id(ink_e2e::AccountKeyring::Bob);

            let constructor = ILOCKmvpRef::new_token();
            let contract_acct_id = client
                .instantiate("ilockmvp", &ink_e2e::alice(), constructor, 0, None)
                .await.expect("instantiate failed").account_id;

            // messages the pay from various pools
            let partners_pay_msg = build_message::<ILOCKmvpRef>(contract_acct_id.clone())
                .call(|contract| contract.payout_tokens(
                    bob_account.clone(), 1000, "PARTNERS".to_string()));
            let whitelist_pay_msg = build_message::<ILOCKmvpRef>(contract_acct_id.clone())
                .call(|contract| contract.payout_tokens(
                    bob_account.clone(), 1000, "WHITELIST".to_string()));
            let publicsale_pay_msg = build_message::<ILOCKmvpRef>(contract_acct_id.clone())
                .call(|contract| contract.payout_tokens(
                    bob_account.clone(), 1000, "PUBLIC_SALE".to_string()));

            // alice pays 1000 ILOCK to bob from PARTNERS pool
            let _partners_pay_result = client
                .call(&ink_e2e::alice(), partners_pay_msg, 0, None).await;

            // checks that alice has expected resulting balance
            let alice_balance_msg = build_message::<ILOCKmvpRef>(contract_acct_id.clone())
                .call(|contract| contract.balance_of(alice_account.clone()));
            let mut alice_balance = client
                .call_dry_run(&ink_e2e::alice(), &alice_balance_msg, 0, None).await.return_value();
            assert_eq!(SUPPLY_CAP - 1000, alice_balance);

            // checks that bob has expected resulting balance
            let bob_balance_msg = build_message::<ILOCKmvpRef>(contract_acct_id.clone())
                .call(|contract| contract.balance_of(bob_account.clone()));
            let mut bob_balance = client
                .call_dry_run(&ink_e2e::alice(), &bob_balance_msg, 0, None).await.return_value();
            assert_eq!(0 + 1000, bob_balance);

            // checks that pool has expected resulting balance
            let mut pool_balance_msg = build_message::<ILOCKmvpRef>(contract_acct_id.clone())
                .call(|contract| contract.pool_balance(PARTNERS));
            let mut pool_balance = client
                .call_dry_run(&ink_e2e::alice(), &pool_balance_msg, 0, None).await.return_value().1;
            assert_eq!(POOLS[PARTNERS as usize].tokens * DECIMALS_POWER10 - 1000, pool_balance);

            // alice pays 1000 ILOCK to bob from WHITELIST pool
            let _whitelist_pay_result = client
                .call(&ink_e2e::alice(), whitelist_pay_msg, 0, None).await;

            // checks that alice has expected resulting balance
            alice_balance = client
                .call_dry_run(&ink_e2e::alice(), &alice_balance_msg, 0, None).await.return_value();
            assert_eq!(SUPPLY_CAP - 1000 - 1000, alice_balance);

            // checks that bob has expected resulting balance
            bob_balance = client
                .call_dry_run(&ink_e2e::alice(), &bob_balance_msg, 0, None).await.return_value();
            assert_eq!(0 + 1000 + 1000, bob_balance);

            // checks that pool has expected resulting balance
            pool_balance_msg = build_message::<ILOCKmvpRef>(contract_acct_id.clone())
                .call(|contract| contract.pool_balance(WHITELIST));
            pool_balance = client
                .call_dry_run(&ink_e2e::alice(), &pool_balance_msg, 0, None).await.return_value().1;
            assert_eq!(POOLS[WHITELIST as usize].tokens * DECIMALS_POWER10 - 1000, pool_balance);

            // alice pays 1000 ILOCK to bob from PUBLIC_SALE pool
            let _publicsale_pay_result = client
                .call(&ink_e2e::alice(), publicsale_pay_msg, 0, None).await;

            // checks that alice has expected resulting balance
            alice_balance = client
                .call_dry_run(&ink_e2e::alice(), &alice_balance_msg, 0, None).await.return_value();
            assert_eq!(SUPPLY_CAP - 1000 - 1000 - 1000, alice_balance);

            // checks that bob has expected resulting balance
            bob_balance = client
                .call_dry_run(&ink_e2e::alice(), &bob_balance_msg, 0, None).await.return_value();
            assert_eq!(0 + 1000 + 1000 + 1000, bob_balance);

            // checks that pool has expected resulting balance
            pool_balance_msg = build_message::<ILOCKmvpRef>(contract_acct_id.clone())
                .call(|contract| contract.pool_balance(PUBLIC_SALE));
            pool_balance = client
                .call_dry_run(&ink_e2e::alice(), &pool_balance_msg, 0, None).await.return_value().1;
            assert_eq!(POOLS[PUBLIC_SALE as usize].tokens * DECIMALS_POWER10 - 1000, pool_balance);

            Ok(())
        }

        /// SAD PAYOUT_TOKENS
        /// - Checks to make sure payout_tokens function fails as expected.
        ///
        /// - Return
        ///     CallerNotOwner               - when caller does not own contract
        ///     InvalidPool                  - when pool isn't (PARTNERS|WHITELIST|PUBLIC_SALE)
        ///     PaymentTooLarge              - when specified payment amount is more than pool
        #[ink_e2e::test]
        async fn sade2e_payout_tokens(
            mut client: ink_e2e::Client<C, E>,
        ) -> E2EResult<()> {

            Ok(())
        }

        /// HAPPY REWARD_INTERLOCKER
        /// - Test if rewarding functionality works.
        /// - Update rewardedtotal.
        /// - Transfer reward amount from rewards pool to Interlocker.
        /// - Update individual rewardedinterlockertotal
        /// - Emit reward event.
        /// - Return new interlocker rewarded total.
        /// - Test that rewarded_total() works.
        /// - Test that rewarded_interlocker_total() works.
        #[ink_e2e::test]
        async fn happye2e_reward_interlocker(
            mut client: ink_e2e::Client<C, E>,
        ) -> E2EResult<()> {

            let alice_account = ink_e2e::account_id(ink_e2e::AccountKeyring::Alice);
            let bob_account = ink_e2e::account_id(ink_e2e::AccountKeyring::Bob);

            let constructor = ILOCKmvpRef::new_token();
            let contract_acct_id = client
                .instantiate("ilockmvp", &ink_e2e::alice(), constructor, 0, None)
                .await.expect("instantiate failed").account_id;

            // alice rewards bob the happy interlocker 1000 ILOCK
            let alice_reward_msg = build_message::<ILOCKmvpRef>(contract_acct_id.clone())
                .call(|contract| contract.reward_interlocker(1000, bob_account.clone()));
            let reward_response = client
                .call(&ink_e2e::alice(), alice_reward_msg, 0, None).await.unwrap();
            
            // filter for reward event
            let contract_emitted_reward = reward_response
                .events
                .iter()
                .find(|event| {
                    event
                        .as_ref()
                        .expect("expected event")
                        .event_metadata()
                        .event()
                        == "ContractEmitted" &&
                        String::from_utf8_lossy(
                            event.as_ref().expect("bad event").bytes()).to_string()
                       .contains("ILOCKmvp::Reward")
                })
                .expect("Expect ContractEmitted event")
                .unwrap();

            // decode to the expected event type (skip field_context)
            let reward_event = contract_emitted_reward.field_bytes();
            let decoded_reward =
                <Reward as scale::Decode>::decode(&mut &reward_event[34..]).expect("invalid data");

            // destructor decoded transfer
            let Reward { to, amount } = decoded_reward;

            // assert with the expected value
            assert_eq!(to, Some(bob_account), "encountered invalid Reward.to");
            assert_eq!(amount, 1000, "encountered invalid Reward.amount");  

            // checks that alice has expected resulting balance
            let alice_balance_msg = build_message::<ILOCKmvpRef>(contract_acct_id.clone())
                .call(|contract| contract.balance_of(alice_account.clone()));
            let alice_balance = client
                .call_dry_run(&ink_e2e::alice(), &alice_balance_msg, 0, None).await.return_value();
            assert_eq!(SUPPLY_CAP - 1000, alice_balance);

            // checks that pool has expected resulting balance
            let pool_balance_msg = build_message::<ILOCKmvpRef>(contract_acct_id.clone())
                .call(|contract| contract.pool_balance(REWARDS));
            let pool_balance = client
                .call_dry_run(&ink_e2e::alice(), &pool_balance_msg, 0, None).await.return_value().1;
            assert_eq!(POOLS[REWARDS as usize].tokens * DECIMALS_POWER10 - 1000, pool_balance);

            // checks that bob has expected resulting balance
            let bob_balance_msg = build_message::<ILOCKmvpRef>(contract_acct_id.clone())
                .call(|contract| contract.balance_of(bob_account.clone()));
            let bob_balance = client
                .call_dry_run(&ink_e2e::alice(), &bob_balance_msg, 0, None).await.return_value();
            assert_eq!(0 + 1000, bob_balance);

            // checks that circulating supply was properly incremented
            let total_supply_msg = build_message::<ILOCKmvpRef>(contract_acct_id.clone())
                .call(|contract| contract.total_supply());
            let total_supply = client
                .call_dry_run(&ink_e2e::alice(), &total_supply_msg, 0, None).await.return_value();
            assert_eq!(0 + 1000, total_supply);

            // checks that total rewarded (overall) is correct
            let total_rewarded_msg = build_message::<ILOCKmvpRef>(contract_acct_id.clone())
                .call(|contract| contract.rewarded_total());
            let total_rewarded = client
                .call_dry_run(&ink_e2e::alice(), &total_rewarded_msg, 0, None).await.return_value();
            assert_eq!(0 + 1000, total_rewarded);

            // checks that total rewarded (to interlocker) is correct
            let total_rewarded_interlocker_msg = build_message::<ILOCKmvpRef>(contract_acct_id.clone())
                .call(|contract| contract.rewarded_interlocker_total(bob_account.clone()));
            let total_rewarded_interlocker = client
                .call_dry_run(&ink_e2e::alice(), &total_rewarded_interlocker_msg, 0, None).await.return_value();
            assert_eq!(0 + 1000, total_rewarded_interlocker);

            Ok(())
        }

        /// SAD REWARD_INTERLOCKER
        /// - Test if rewarding functionality fails correctly.
        ///
        /// - Return
        ///     CallerNotOwner               - when caller does not own contract
        ///     PaymentTooLarge              - when arithmetic over or underflows
        ///
        ///     ... maybe check the over/underflows?
        #[ink_e2e::test]
        async fn sade2e_reward_interlocker(
            mut client: ink_e2e::Client<C, E>,
        ) -> E2EResult<()> {

            Ok(())
        } 
    }

////////////////////////////////////////////////////////////////////////////
//// unit tests ////////////////////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////

    #[cfg(test)]
    mod tests {

        use ink::primitives::Hash;

        use super::*;

        /// - Test if the default constructor does its job
        /// - and check months_passed()
        /// - and check cap().
        #[ink::test]
        fn new_token_works() {

            let ILOCKmvpPSP22 = ILOCKmvp::new_token();

            assert_eq!(ILOCKmvpPSP22.vest.monthspassed, ILOCKmvpPSP22.months_passed());
            assert_eq!(ILOCKmvpPSP22.vest.nextpayout, ILOCKmvpPSP22.env().block_timestamp() + ONE_MONTH);
            assert_eq!(ILOCKmvpPSP22.total_supply(), 0);
            assert_eq!(ILOCKmvpPSP22.metadata.name, Some("Interlock Network".as_bytes().to_vec()));
            assert_eq!(ILOCKmvpPSP22.metadata.symbol, Some("ILOCK".as_bytes().to_vec()));
            assert_eq!(ILOCKmvpPSP22.metadata.decimals, 18);

            // this checks that token numbers have been entered accurately into POOLS PoolData
            let mut total_tokens: u128 = 0;
            for pool in 0..POOL_COUNT {

                total_tokens += POOLS[pool].tokens * DECIMALS_POWER10;
            }
            assert_eq!(total_tokens, ILOCKmvpPSP22.cap());
            assert_eq!(ILOCKmvpPSP22.ownable.owner, ILOCKmvpPSP22.env().caller());
        }

        /// HAPPY REGISTER_STAKEHOLDER & STAKEHOLDER_DATA
        /// - Test if register_stakeholder and stakeholder_data functions works correctly.
        /// - Registration should succeed as long as stakeholder share > 0.
        /// - Payremaining should accurately reflect distribution to stakeholder given share.
        #[ink::test]
        fn happyunit_register_stakeholder_data() {

        }    

        /// HAPPY POOL_DATA AND POOL_BALANCE
        /// - Test if pool_data getter does its job.
        /// - Test if pool_balance does its job.
        #[ink::test]
        fn happyunit_pool_data_and_balance() {

            let ILOCKmvpPSP22 = ILOCKmvp::new_token();
            let pool = &POOLS[1];
            assert_eq!(ILOCKmvpPSP22.pool_data(1), (
                format!("pool: {:?} ", pool.name.to_string()),
                format!("tokens alotted: {:?} ", pool.tokens),
                format!("number of vests: {:?} ", pool.vests),
                format!("vesting cliff: {:?} ", pool.cliffs),
            ));
        }

        /// HAPPY CREATE_GET_PORT
        /// - Test if create_port() and port() functions correctly.
        /// - Test if tax_port_transfer() functions correctly.
        #[ink::test]
        fn happyunit_create_get_port_tax_transfer() {

            let mut ILOCKmvpPSP22 = ILOCKmvp::new_token();
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            let codehash: Hash = Default::default(); // offchain environment doesn't support
            let tax: Balance = 1_000; // 10% tax      // .own_code_hash()
            let cap: Balance = 1_000_000;
            let locked: bool = true;
            let number: u16 = 2;
            let owner: AccountId = accounts.bob;

            let _ = ILOCKmvpPSP22.create_port(
                codehash,
                tax,
                cap,
                locked,
                number,
                owner,
            );

            let mut port: Port = ILOCKmvpPSP22.port(number);

            assert_eq!(port, Port {
                application: codehash,
                tax: tax,
                cap: cap,
                locked: locked,
                paid: 0,
                collected: 0,
                owner: owner,
            });

            ILOCKmvpPSP22.pool.circulating += 1_000_000;

            let test_socket: Socket = Socket {

                operator: accounts.eve,
                portnumber: 2,
            };

            let _ = ILOCKmvpPSP22.tax_port_transfer(
                test_socket,
                port,
                cap,
            );

            port = ILOCKmvpPSP22.app.ports.get(number).unwrap();

            assert_eq!(port.paid, 1_000_000 - 1_000); // 999_000
            assert_eq!(port.collected, 0 + 1_000);
            assert_eq!(ILOCKmvpPSP22.proceeds_available(), 0 + 1_000);
            assert_eq!(ILOCKmvpPSP22.total_supply(), 1_000_000 - 1_000);
        }

        /// SAD TAX_PORT_TRANSFER
        /// - Not sure there is much to do here.
        #[test]
        fn sadunit_tax_port_transfer() {
        }

/*************************  THIS TEST IS SLOW, THUS COMMENTED OUT UNLESS NEEDED

        /// HAPPY CHECK_TIME
        /// - Test to make sure month increment doesn't happen too soon.
        #[ink::test]
        fn happyunit_check_time() {

            let mut ILOCKmvpPSP22 = ILOCKmvp::new_token();

            for _time in 0..432_000_001 { // number of advances needed to span month

                ink::env::test::advance_block::<ink::env::DefaultEnvironment>();
            }
            let timestamp: Timestamp = ink::env::block_timestamp::<ink::env::DefaultEnvironment>();

            assert!(ILOCKmvpPSP22.vest.nextpayout < timestamp);
            assert_eq!(ILOCKmvpPSP22.vest.monthspassed, 0);
            let _ = ILOCKmvpPSP22.check_time();
            assert_eq!(ILOCKmvpPSP22.vest.monthspassed, 1);
        }

**************************/
    }
}
