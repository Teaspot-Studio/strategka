use log::warn;
use nom::{error::context, number::complete::be_u64, Err, IResult};
use serde::de::DeserializeOwned;

use super::error::Error;

pub type Parser<'a, T> = IResult<&'a [u8], T, Error<'a>>;

pub fn length_decoding<'a, R, F>(f: F) -> impl FnMut(&'a [u8]) -> Parser<'a, Option<R>>
where
    F: FnMut(&'a [u8]) -> Parser<'a, R> + Copy,
{
    move |input| {
        let (input, len) = context("block length", be_u64)(input)?;
        if input.len() < len as usize {
            return Err(Err::Error(Error::InvalidLength(len as usize, input.len())));
        }
        let restricted_input = &input[0..len as usize];
        let result = if len == 0 {
            warn!("Block length is 0");
            None
        } else {
            let (_, result) = context("block body", f)(restricted_input)?;
            Some(result)
        };
        Ok((&input[len as usize..], result))
    }
}

pub fn decode_vec<'a, R, F>(item_parser: F) -> impl FnMut(&'a [u8]) -> Parser<'a, Vec<R>>
where
    F: FnMut(&'a [u8]) -> Parser<'a, R> + Copy,
{
    move |input| {
        let (input, len) = context("vector length", be_u64)(input)?;
        let mut result = Vec::with_capacity(len as usize);
        let mut cycle_input = input;
        for _ in 0..len {
            let (input, item) = context("vector item", item_parser)(cycle_input)?;
            cycle_input = input;
            result.push(item);
        }
        Ok((cycle_input, result))
    }
}

pub fn ciborium_parse<'a, T: DeserializeOwned>(input: &'a [u8]) -> Parser<'a, T> {
    let res = ciborium::de::from_reader(input)
        .map_err(Error::Decoder)
        .map_err(Err::Failure)?;
    Ok((&input[input.len()..], res))
}
