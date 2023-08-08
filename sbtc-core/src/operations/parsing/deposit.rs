/*!
Deposit is a transaction with the output structure as below:

1. data output
2. payment to peg wallet address

The data output should contain data in the following format:

0      2  3                  24                            64       80
|------|--|------------------|-----------------------------|--------|
 magic  op   Stacks address      Contract name (optional)     memo
```
*/
use std::str::from_utf8;

use stacks_core::{
    address::{AddressVersion, StacksAddress},
    crypto::{hash160::Hash160Hasher, Hashing},
    utils::{ContractName, PrincipalData, StandardPrincipalData},
};

use crate::{SBTCError, SBTCResult};

fn find_leading_non_zero_bytes(data: &[u8]) -> Option<&[u8]> {
    match data.iter().rev().position(|&b| b != 0) {
        Some(end) if end != 0 => Some(&data[0..=end]),
        Some(_) | None => None,
    }
}

/// Parsed data output from a deposit transaction
pub struct ParsedDepositData {
    /// The recipient of the deposit
    pub recipient: PrincipalData,
    /// The memo
    pub memo: Vec<u8>,
}

/// Parses the subset of the data output from a deposit transaction. First 3 bytes need to be removed.
pub fn parse(data: &[u8]) -> SBTCResult<ParsedDepositData> {
    if data.len() < 21 {
        return Err(SBTCError::MalformedData("Should contain at least 21 bytes"));
    }

    let standard_principal_data = {
        let version = AddressVersion::from_repr(*data.first().expect("No version byte in data"))
            .ok_or(SBTCError::MalformedData("Address version is invalid"))?;
        let address_data: [u8; 20] = data
            .get(1..21)
            .ok_or(SBTCError::MalformedData("Could not get address data"))?
            .try_into()
            .map_err(|_| {
                SBTCError::MalformedData("Byte data is larger than 20 bytes for the address")
            })?;

        StandardPrincipalData::new(
            version,
            StacksAddress::new(version, Hash160Hasher::new(address_data)),
        )
    };

    let recipient = find_leading_non_zero_bytes(&data[21..=61])
        .map(|contract_bytes| {
            let contract_name_string: String = from_utf8(contract_bytes)
                .map_err(|_| SBTCError::MalformedData("Could not parse contract name bytes"))?
                .to_owned();
            let contract_name = ContractName::new(&contract_name_string)
                .map_err(|_| SBTCError::MalformedData("Could not parse contract name"))?;

            Result::<_, SBTCError>::Ok(PrincipalData::Contract(
                standard_principal_data.clone(),
                contract_name,
            ))
        })
        .unwrap_or(Ok(PrincipalData::Standard(standard_principal_data)))?;

    let memo = data.get(61..).unwrap_or(&[]).to_vec();

    Ok(ParsedDepositData { recipient, memo })
}