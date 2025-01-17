//!
//! # INTERLOCK NETWORK MVP SMART CONTRACT
//!  - ### PSP22 TOKEN
//!  - ### REWARDS
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
//! echo "<meta http-equiv=\"refresh\" content=\"0; url=ilockmvp\">" >
//! target/doc/index.html;
//! cp -r target/doc ./docs
//!

#![doc(
    html_logo_url = "https://uploads-ssl.webflow.com/6293b370c2da3eda80121e92/6293d7cffa42ae33001294d1_interlock-visual-hero.png",
    html_favicon_url = "https://uploads-ssl.webflow.com/6293b370c2da3eda80121e92/6293d7cffa42ae33001294d1_interlock-visual-hero.png",
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
                extensions::metadata::*,
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
    pub const POOL_COUNT: usize = 13;                               // number of token pools
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
        pub name: &'a str,
        pub tokens: u128,
        pub vests: u8,
        pub cliffs: u8,
    }

    /// - Pool data.
    pub const POOLS: [PoolData; POOL_COUNT] = [
        PoolData { name: "presale_1",                     tokens: 48_622_222,  vests: 18, cliffs: 1, },
        PoolData { name: "presale_2",                     tokens: 33_333_333,  vests: 15, cliffs: 1, },
        PoolData { name: "presale_3",                     tokens: 93_750_000,  vests: 12, cliffs: 1, },
        PoolData { name: "team+founders",                 tokens: 200_000_000, vests: 36, cliffs: 6, },
        PoolData { name: "outlier_ventures",              tokens: 40_000_000,  vests: 24, cliffs: 1, },
        PoolData { name: "advisors",                      tokens: 25_000_000,  vests: 24, cliffs: 1, },
        PoolData { name: "foundation",                    tokens: 169_264_142, vests: 84, cliffs: 1, },
        PoolData { name: "rewards",                       tokens: 300_000_000, vests: 48, cliffs: 0, },
        PoolData { name: "partners",                      tokens: 37_000_000,  vests: 1,  cliffs: 0, },
        PoolData { name: "community_sale",                tokens: 3_030_303,   vests: 1,  cliffs: 0, },
        PoolData { name: "public_sale",                   tokens: 50_000_000,  vests: 1,  cliffs: 0, },
        PoolData { name: "proceeds",                      tokens: 0,           vests: 0,  cliffs: 0, },
        PoolData { name: "circulating",                   tokens: 0,           vests: 0,  cliffs: 0, },
    ];

    /// - Pools.
    pub const PRESALE_1: u8         = 0;
    pub const PRESALE_2: u8         = 1;
    pub const PRESALE_3: u8         = 2;
    pub const TEAM: u8              = 3;
    pub const OUTLIER: u8           = 4;
    pub const ADVISORS: u8          = 5;
    pub const FOUNDATION: u8        = 6;
    pub const REWARDS: u8           = 7;
    pub const PARTNERS: u8          = 8;
    pub const COMMUNITY: u8         = 9;
    pub const PUBLIC: u8            = 10;
    pub const PROCEEDS: u8          = 11;
    pub const CIRCULATING: u8       = 12;

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
        pub paid: Balance,

        /// - What is the overall ILOCK token share for this stakeholder?
        pub share: Balance,

        /// - Which vesting pool does this stakeholder belong to?
        /// - The pool determines the vesting schedule.
        pub pool: u8,
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
        pub application: Hash,

        /// - How much does Interlock tax transaction taking place within a port protocol's
        /// socket call?
        pub tax: Balance,

        /// - For withdrawing rewards from ILOCK rewards pool, what is the max this particular
        /// port owner's application type can withdraw from rewards pool?
        pub cap: Balance,

        /// - If locked, only Interlock token contract owner can create a socket connection with
        /// this token contract using the appropriate application codehash.
        pub locked: bool,

        /// - How much ILOCK has this port been rewarded or issued throughout the course of
        /// its operation (in case where protocol rewards or issues ILOCK, that is)?
        pub paid: Balance,

        /// - How much has Interlock collected from this port in taxes or other collections?
        pub collected: Balance,

        /// - Who is the overall owner of this port?
        /// - Socket operators are not necessarily owners of the port.
        /// - For example, a restaurant franchise has one owner, whereas the franchise may have
        /// numberous restaurant locations, each with it's own operator, each operator/franchise
        /// pair forming a separate socket connection.
        pub owner: AccountId,
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
        pub operator: AccountId,

        /// - What port is this operator connected to?
        /// - Using the restaurant franchise metaphor again, the port is like the franchise
        /// itself.
        /// - The port number is what identifies a particular franchise and its protocols,
        /// procedures, metadata, and ultimately business model and standards for any
        /// franchisees.
        pub portnumber: u16,
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
        pub psp22: psp22::Data,

        /// - Openbrush ownership extension.
        #[storage_field]
        pub ownable: ownable::Data,

        /// - Openbrush metadata extension.
        #[storage_field]
        pub metadata: metadata::Data,

        /// - ILOCK Rewards info.
        #[storage_field]
        pub reward: RewardData,

        /// - ILOCK vesting info.
        #[storage_field]
        pub vest: VestData,

        /// - ILOCK connecting application contract info
        #[storage_field]
        pub app: AppData,

        /// - ILOCK token pool balances.
        pub balances: [Balance; POOL_COUNT],
    }

////////////////////////////////////////////////////////////////////////////
//// events and errors /////////////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////

    /// - Specify transfer event.
    #[ink(event)]
    pub struct Transfer {
        #[ink(topic)]
        pub from: Option<AccountId>,
        #[ink(topic)]
        pub to: Option<AccountId>,
        pub amount: Balance,
    }

    /// - Specify approval event.
    #[ink(event)]
    pub struct Approval {
        #[ink(topic)]
        pub owner: Option<AccountId>,
        #[ink(topic)]
        pub spender: Option<AccountId>,
        pub amount: Balance,
    }

    /// - Specify reward event.
    #[ink(event)]
    pub struct Reward {
        #[ink(topic)]
        pub to: Option<AccountId>,
        pub amount: Balance,
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
        /// - Returned if transfer caller is the owner.
        CallerIsOwner,
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
            self.balances[CIRCULATING as usize]
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

            // if sender is owner, deny
            if from == self.ownable.owner {

               return Err(OtherError::CallerIsOwner.into()); 
            }

            let _ = self._transfer_from_to(from, to, value, data)?;

            // if recipient is owner, then tokens are being returned or added to rewards pool
            if to == self.ownable.owner {

                match self.balances[REWARDS as usize].checked_add(value) {
                    Some(sum) => self.balances[REWARDS as usize] = sum,
                    None => return Err(OtherError::Overflow.into()),
                };
                match self.balances[CIRCULATING as usize].checked_sub(value) {
                    Some(difference) => self.balances[CIRCULATING as usize] = difference,
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

                match self.balances[CIRCULATING as usize].checked_add(value) {
                    Some(sum) => self.balances[CIRCULATING as usize] = sum,
                    None => return Err(OtherError::Overflow.into()),
                };
            }

            // if recipient is owner, then tokens are being returned or added to rewards pool
            if to == self.ownable.owner {

                match self.balances[REWARDS as usize].checked_add(value) {
                    Some(sum) => self.balances[REWARDS as usize] = sum,
                    None => return Err(OtherError::Overflow.into()),
                };
                match self.balances[CIRCULATING as usize].checked_sub(value) {
                    Some(difference) => self.balances[CIRCULATING as usize] = difference,
                    None => return Err(OtherError::Underflow.into()),
                };
            }

            Ok(())
        }
    }

    impl PSP22Metadata for ILOCKmvp {}

    impl Ownable for ILOCKmvp {
        
        /// - Override default transfer ownership function.
        /// - On transfer, also transfer old owner balance to new owner.
        /// - That is, transfer noncirculating tokens to new owner.
        #[ink(message)]
        fn transfer_ownership(
            &mut self,
            newowner: AccountId
        ) -> Result<(), OwnableError> {

            let oldowner = self.ownable.owner;
            
            // only-owner modifier not present in this scope
            if oldowner != self.env().caller() {

                return Err(OwnableError::CallerIsNotOwner);
            }
            let oldbalance: Balance = self.balance_of(oldowner);

            // transfer all remaining owner tokens (pools) to new owner
            let mut newbalance: Balance = self.psp22.balance_of(newowner);
            match newbalance.checked_add(oldbalance) {
                Some(sum) => newbalance = sum,
                None => (), // case not possible
            };
            self.psp22.balances.insert(&newowner, &newbalance);

            // deduct tokens from owners account
            self.psp22.balances.insert(&oldowner, &0);

            self.ownable.owner = newowner;

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
    /// - This is necessary because a struct in PSP34 needs derive(Default)
    /// and the contract Ref has no derivable Default implementation.
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

            contract.metadata.name = Some(TOKEN_NAME.to_string().into_bytes());
            contract.metadata.symbol = Some(TOKEN_SYMBOL.to_string().into_bytes());
            contract.metadata.decimals = TOKEN_DECIMALS;

            // mint with openbrush:
            contract._mint_to(caller, SUPPLY_CAP)
                    .expect("Failed to mint the initial supply");
            contract._init_with_owner(caller);

            // create initial pool balances
            for pool in 0..POOL_COUNT {

                contract.balances[pool] =
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
        /// - Data used to calculate monthly payouts and track net paid.
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
        /// - Returns tuple (StakeholderData, payremaining, payamount, poolname).
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

        /// - General function to transfer share a stakeholder is currently entitled to.
        /// - This is called once per stakeholder per month by Interlock, Interlock paying fees.
        /// - Pools are guaranteed to have enough tokens for all stakeholders.
        #[ink(message)]
        #[openbrush::modifiers(only_owner)]
        pub fn distribute_tokens(
            &mut self,
            stakeholder: AccountId,
        ) -> OtherResult<()> {

            // get data structs
            let mut this_stakeholder = match self.vest.stakeholder.get(stakeholder) {
                Some(s) => s,
                None => { return Err(OtherError::StakeholderNotFound) },
            };
            let pool = &POOLS[this_stakeholder.pool as usize];

            // require cliff to have been surpassed
            if self.vest.monthspassed < pool.cliffs as u16 {
                return Err(OtherError::CliffNotPassed)
            }

            // require share has not been completely paid out
            if this_stakeholder.paid == this_stakeholder.share {
                return Err(OtherError::StakeholderSharePaid)
            }

            // calculate the payout owed
            // ! no checked_div needed; pool.vests guaranteed to be nonzero
            let mut payout: Balance = this_stakeholder.share / pool.vests as Balance;

            // require that payout isn't repeatable for this month
            // ! no checked_div needed; this_stakeholder.share guaranteed to be nonzero
            let payments = this_stakeholder.paid / payout;
            if payments >= self.vest.monthspassed as u128 {
                return Err(OtherError::PayoutTooEarly)
            }

            // calculate the new total paid to stakeholder
            let mut newpaidtotal: Balance = match this_stakeholder.paid.checked_add(payout) {
                Some(sum) => sum,
                None => return Err(OtherError::Overflow),
            };

            // calculate remaining share
            let remainingshare: Balance = match this_stakeholder.share.checked_sub(newpaidtotal) {
                Some(difference) => difference,
                None => return Err(OtherError::Underflow),
            };

            // if this is final payment, add token remainder to payout
            // (this is to compensate for floor division that calculates payamount)
            if remainingshare < payout {

                payout += remainingshare;
                newpaidtotal = this_stakeholder.share;
            }

            // increment distribution to stakeholder account
            let mut stakeholderbalance: Balance = self.psp22.balance_of(stakeholder);
            match stakeholderbalance.checked_add(payout) {
                Some(sum) => stakeholderbalance = sum,
                None => return Err(OtherError::Overflow),
            };
            self.psp22.balances.insert(&stakeholder, &stakeholderbalance);

            // increment total supply
            match self.balances[CIRCULATING as usize].checked_add(payout) {
                Some(sum) => self.balances[CIRCULATING as usize] = sum,
                None => return Err(OtherError::Overflow),
            };

            // deduct tokens from owners account
            let mut ownerbalance: Balance = self.psp22.balance_of(self.env().caller());
            match ownerbalance.checked_sub(payout) {
                Some(difference) => ownerbalance = difference,
                None => return Err(OtherError::Underflow),
            };
            self.psp22.balances.insert(&self.env().caller(), &ownerbalance);

            // update pool balance
            match self.balances[this_stakeholder.pool as usize].checked_sub(payout) {
                Some(difference) => self.balances[this_stakeholder.pool as usize] = difference,
                None => return Err(OtherError::Underflow),
            };

            // finally update stakeholder data struct state
            this_stakeholder.paid = newpaidtotal;
            self.vest.stakeholder.insert(stakeholder, &this_stakeholder);

            Ok(())
        }

        /// - Function used to payout tokens to pools with no vesting schedule.
        /// POOL ARGUMENTS:
        ///      PARTNERS
        ///      COMMUNITY
        ///      PUBLIC
        ///      PROCEEDS
        #[ink(message)]
        #[openbrush::modifiers(only_owner)]
        pub fn payout_tokens(
            &mut self,
            stakeholder: AccountId,
            amount: Balance,
            pool: String,
        ) -> OtherResult<()> {

            let owner: AccountId = self.env().caller();

            let poolnumber: u8 = match pool.as_str() {
                "PARTNERS"      => 8,
                "COMMUNITY"     => 9,
                "PUBLIC"        => 10,
                "PROCEEDS"      => 11,
                _ => return Err(OtherError::InvalidPool)
            };

            // deduct payout amount
            match self.balances[poolnumber as usize].checked_sub(amount) {
                Some(difference) => self.balances[poolnumber as usize] = difference,
                None => return Err(OtherError::PaymentTooLarge),
            };

            // now transfer tokens
            // increment distribution to stakeholder account
            let mut stakeholderbalance: Balance = self.psp22.balance_of(stakeholder);
            match stakeholderbalance.checked_add(amount) {
                Some(sum) => stakeholderbalance = sum,
                None => return Err(OtherError::Overflow),
            };
            self.psp22.balances.insert(&stakeholder, &stakeholderbalance);

            // increment total supply
            match self.balances[CIRCULATING as usize].checked_add(amount) {
                Some(sum) => self.balances[CIRCULATING as usize] = sum,
                None => return Err(OtherError::Overflow),
            };

            // deduct tokens from owners account
            let mut ownerbalance: Balance = self.psp22.balance_of(owner);
            match ownerbalance.checked_sub(amount) {
                Some(difference) => ownerbalance = difference,
                None => return Err(OtherError::Underflow),
            };
            self.psp22.balances.insert(&owner, &ownerbalance);

            Ok(())
        }

////////////////////////////////////////////////////////////////////////////
/////// pool data //////////////////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////

        /// - Function that returns pool data in human readable format..
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
        /// - Provide human readable and numberic format.
        #[ink(message)]
        pub fn pool_balance(
            &self,
            poolnumber: u8,
        ) -> (String, Balance) {

            (format!("pool: {:?}, balance: {:?}", 
                    POOLS[poolnumber as usize].name.to_string(),
                    self.balances[poolnumber as usize]),
             self.balances[poolnumber as usize])
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
        ) -> OtherResult<Balance> {

            // make sure reward not too large
            if self.balances[REWARDS as usize] < reward {
                return Err(OtherError::PaymentTooLarge)
            }

            // make sure vest limit will not be passed with this reward
            let monthly: Balance = POOLS[REWARDS as usize].tokens * DECIMALS_POWER10 /
                POOLS[REWARDS as usize].vests as Balance;
            let currentcap: Balance = (self.vest.monthspassed + 1) as Balance * monthly;
            if currentcap < POOLS[REWARDS as usize].tokens * DECIMALS_POWER10
                - self.balances[REWARDS as usize] + reward {

                return Err(OtherError::PayoutTooEarly)
            }

            // make sure rewardee is not contract
            if self.env().is_contract(&interlocker) {
                return Err(OtherError::CannotRewardContract)
            }

            // update rewards pool balance
            // (contract calls transfer, not owner, thus we must update here)
            match self.balances[REWARDS as usize].checked_sub(reward) {
                Some(difference) => self.balances[REWARDS as usize] = difference,
                None => return Err(OtherError::PaymentTooLarge),
            };

            // increment reward to operator's account
            let mut interlockerbalance: Balance = self.psp22.balance_of(interlocker);
            match interlockerbalance.checked_add(reward) {
                Some(sum) => interlockerbalance = sum,
                None => return Err(OtherError::Overflow),
            };
            self.psp22.balances.insert(&interlocker, &interlockerbalance);

            // update total amount rewarded to interlocker
            match self.reward.total.checked_add(reward) {
                Some(sum) => self.reward.total = sum,
                None => return Err(OtherError::PaymentTooLarge),
            };

            // increment total supply
            match self.balances[CIRCULATING as usize].checked_add(reward) {
                Some(sum) => self.balances[CIRCULATING as usize] = sum,
                None => return Err(OtherError::Overflow),
            };

            // deduct tokens from owners account
            let mut ownerbalance: Balance = self.psp22.balance_of(self.env().caller());
            match ownerbalance.checked_sub(reward) {
                Some(difference) => ownerbalance = difference,
                None => return Err(OtherError::Underflow),
            };
            self.psp22.balances.insert(&self.env().caller(), &ownerbalance);

            // compute and update new total awarded to interlocker
            let rewardedinterlockertotal: Balance = match self.reward.interlocker.get(interlocker) {
                Some(total) => total,
                None => 0,
            };
            let newrewardedtotal: Balance = match rewardedinterlockertotal.checked_add(reward) {
                Some(sum) => sum,
                None => return Err(OtherError::PaymentTooLarge),
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
        /// socket (ie, a port must first exist before a socket may form)..
        #[ink(message)]
        pub fn create_socket(
            &mut self,
            operator: AccountId,
            portnumber: u16,
        ) -> OtherResult<()> {

            // get application contract address
            let application: AccountId = self.env().caller();

            // make sure caller is a contract, return if not
            if !self.env().is_contract(&application) {
                return Err(OtherError::NotContract);
            };

            // get hash of calling application contract
            let callinghash: Hash = match self.env().code_hash(&application) {
                Ok(hash) => hash,
                Err(_) => return Err(OtherError::NotContract),
            };

            // get port specified by calling application contract
            let port: Port = match self.app.ports.get(portnumber) {
                Some(port) => port,
                None => return Err(OtherError::NoPort),
            };

            // make sure port is unlocked, or caller is token contract owner (interlock)
            //   . this makes it so that people can't build their own client application
            //     to 'hijack' an approved and registered rewards contract.
            //   . if port is locked then only interlock can create new socket with port
            //   . socket creation is only called by an external application contract that
            //     the port represents
            if port.locked && (self.ownable.owner != operator) {
                return Err(OtherError::PortLocked);
            }
            
            // compare calling contract hash to registered port hash
            // to make sure application contract is safe (ie, approved and audited by interlock)
            if callinghash == port.application {
                
                // if safe, contract is allowed to create socket (socket == operatoraddress:portnumber)
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

                        // this is primarily to serve as a socket setup example
                        // ... for this particular case, port two is probably *locked*
                        //
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
            // safe contract registered by interlock as a 'port application' 
            Err(OtherError::UnsafeContract)
        }

        /// - Check for socket and apply custom logic after being called from application contract.
        /// - Application contract calls its socket to trigger internal logic defined by port.
        /// - Default parameters are address and amount or value.
        /// - Additional parameters may be encoded as _data: Vec<u8>.
        #[ink(message)]
        pub fn call_socket(
            &mut self,
            address: AccountId,
            amount: Balance,
            _data: Vec<u8>,
        ) -> OtherResult<()> {

            // get application contract's address
            let application: AccountId = self.env().caller();

            // make sure caller is contract; only application contracts may call socket
            if !self.env().is_contract(&application) {
                return Err(OtherError::NotContract);
            }

            // get socket, to get port assiciated with socket
            let socket: Socket = match self.app.sockets.get(application) {
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
                    match self.balances[REWARDS as usize].checked_add(amount) {
                        Some(sum) => self.balances[REWARDS as usize] = sum,
                        None => return Err(OtherError::Overflow),
                    };
                    match self.balances[CIRCULATING as usize].checked_sub(amount) {
                        Some(difference) => self.balances[CIRCULATING as usize] = difference,
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

            // compute tax - tax number is in centipercent, 0.01% ==> 100% = 1 & 1% = 100 & 0.01% = 10_000
            //
            // a tax of 0.01% is amount/10_000
            let tax: Balance = match amount.checked_div(port.tax as Balance) {
                Some(quotient) => quotient,
                None => return Err(OtherError::DivError),
            };

            // update proceeds pool and total circulation
            match self.balances[PROCEEDS as usize].checked_add(tax) {
                Some(sum) => self.balances[PROCEEDS as usize] = sum,
                None => return Err(OtherError::Overflow),
            };
            match self.balances[CIRCULATING as usize].checked_sub(tax) {
                Some(difference) => self.balances[CIRCULATING as usize] = difference,
                None => return Err(OtherError::Underflow),
            };

            // increment ILOCK contract owner's account balance
            let mut ownerbalance: Balance = self.psp22.balance_of(self.ownable.owner);
            match ownerbalance.checked_add(tax) {
                Some(sum) => ownerbalance = sum,
                None => return Err(OtherError::Overflow),
            };
            self.psp22.balances.insert(&self.ownable.owner, &ownerbalance);

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
//// testing helper ////////////////////////////////////////////////////////
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
 }

#[cfg(all(test, feature = "e2e-tests"))]
pub mod tests_e2e;

#[cfg(test)]
pub mod tests_unit;

// TEST TODO
// in order of appearance
//
// [x] happyunit_total_supply                <-- checked within new_token()
// [x] happye2e_transfer             \
// [] sade2e_transfer                |
// [x] happye2e_transfer_from        |---- we test these because we change the default openbrush
// [] sade2e_transfer_from           /     implementations ... per agreement with Kudelski, we will
//                                         be assuming that openbrush is safe ... we may wish to perform
//                                         additional tests once audit is underway or/ in general future
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
// [x] ** happye2e_create_socket     \
// [x] ** sade2e_create_socket       |----- these must be performed from generic port
// [x] ** happye2e_call_socket       |      or from the uanft contract's self minting message
// [x] ** sade2e_call_socket         /
// [x] happyunit_tax_port_transfer
// [] sadunit_tax_port_transfer
// [x] happyunit_check_time
//
