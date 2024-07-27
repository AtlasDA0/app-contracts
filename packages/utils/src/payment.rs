use cosmwasm_std::{Coin, MessageInfo, StdError, StdResult};

pub fn assert_payment(msg_info: &MessageInfo, expected_coins: Vec<Coin>) -> StdResult<Coin> {
    if msg_info.funds.len() != 1 {
        return Err(StdError::generic_err(format!(
            "You need to send exactly one coin for payment, received {:?}",
            msg_info.funds,
        )));
    }
    let sent_coin = &msg_info.funds[0];

    let payment_coin = expected_coins.iter().find(|c| c.denom == sent_coin.denom);

    let Some(payment_coin) = payment_coin else {
        return Err(StdError::generic_err(format!(
            "Invalid fee payment sent. Expected {:?}, sent {:?}",
            expected_coins, msg_info.funds
        )));
    };

    if payment_coin.amount > sent_coin.amount {
        return Err(StdError::generic_err(format!(
            "Invalid fee payment sent. Expected {}, sent {:?}",
            payment_coin, msg_info.funds
        )));
    }

    Ok(sent_coin.clone())
}
