/****************************************************************
 * INTR Solana Contract
 ****************************************************************/

#![allow(non_snake_case)]
use solana_program::{
        account_info::{
            next_account_info,
            AccountInfo
        },
        entrypoint::ProgramResult,
        program_error::ProgramError,
        program_pack::Pack,
        pubkey::Pubkey,
        sysvar::{
            rent::Rent,
            Sysvar,
        },
        msg,
    };

use bit_vec::BitVec;

use crate::{
        processor::run::Processor,
        utils::utils::*,
        state::{
            GLOBAL::*,
        },
    };

// for this instruction, the expected accounts are
//
// 0, owner pubkey, is signer
// 1, GLOBAL pda
// 2, owner pubkey, or newowner if changing ownership

impl Processor {

    pub fn process_update_global(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        updateFlags: u64,
        values: [u64;64],
    ) -> ProgramResult {

        // iterate and get accounts
        let account_info_iter = &mut accounts.iter();
        let owner = next_account_info(account_info_iter)?;
        let pdaGLOBAL = next_account_info(account_info_iter)?
        let newOwner = next_account_info(account_info_iter)?

        // check to make sure tx sender is signer
        if !owner.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // get GLOBAL account info
        let mut GLOBALinfo = GLOBAL::unpack_unchecked(&pdaGLOBAL.try_borrow_data()?)?;

        // check that owner is *actually* owner
        if GLOBALinfo.owner != owner.key {
            return Err(ContractError::OwnerImposterError.into());
        }

        // if newOwner is different than owner, set new owner and return
        if owner.key != newOwner.key {
            GLOBALinfo.owner = newOwner.key;
            GLOBAL::pack(GLOBALinfo, &mut pdaGLOBAL.try_borrow_mut_data()?)?;
            Ok(())
        }
        
        // unpack ix data flags specifying which global variable to update
        let mut flags = unpack_flags(updateFlags);


        // . check for values that need to be updated
        // . flag high => value is to be changed
        let mut i = 0;
        for flag in flags {
            if flag {GLOBALinfo.values[i] = values[i]}
            i += 1;
        }

        // populate and pack GLOBAL account info
        GLOBALinfo.flags = pack_flags(flags);
        GLOBALinfo.owner = *owner.key;
        GLOBAL::pack(GLOBALinfo, &mut first.try_borrow_mut_data()?)?;

        Ok(())
    }
}

