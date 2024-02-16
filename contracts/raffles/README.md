# Sg-Raffles

This handles the creation & lifecycle of raffles on Stargaze.


## InstantiateMsg

```json
{
    "name": "sg-raffles",
    "nois_proxy_addr": "stars1atcndw8yfrulzux6vg6wtw2c0u4y5wvy9423255h472f4x3gn8dq0v8j45",
    "nois_proxy_amount": "50",
    "nois_proxy_denom": "ustars",
    "creation_fee_amount": "100000000", // static raffle fee
    "creation_fee_denom": [
        "ustars",
        "ibc/ACCAF790E082E772691A20B0208FB972AD3A01C2DE0D7E8C479CCABF6C9F39B1"
    ],
    "fee_addr": "stars1n5x097nd7v8dv8ng4x4xeux5xdv6jas62qslh3",
    "max_ticket_number": 1,
    "minimum_raffle_duration": 120 // in seconds 
}

```

## Contract Actions

### `UpdateConfig`  **admin-only*
*update raffle contract params*
### `CreateRaffle`
*create a raffle.*
### `CancelRaffle`
*cancel a raffle, if no tickets have yet been purchased*
### `ModifyRaffle`
*update raffle settings, if no tickets have yet been purchased*
### `BuyTicket`
*purchase raffle tickets.*
```json
{"buy_ticket":{"raffle_id": 69, "ticket_count": 40, "sent_assets":}}
```
### `Receive`
*internal function to recieve cw721 nfts*
### `DetermineWinner`
*function to calculate winner of raffle*
```json
{"determine_winner":{"raffle_id": 69}}
```
### `UpdateRandomness`
manually request randomness for raffle id
```json
{"update_randomness":{"raffle_id": 69}}
```
### `NoisReceive`
*internal function to recieve nois randomness only from verified source*
### `ToggleLock`   **admin-only* 
*admin function to lock contract actions*

## QueryMsg
### `Config`
*returns contract global configuration*
### `RaffleInfo`
*returns information about the state of a raffle*
```json
{"raffle_info":{"raffle_id": 69}}
```
### `AllRaffles`
*returns information about the state of all raffles*

### `AllTickets`
*batch response of ticket owners*

### `TicketCount`
*responds total tickets purchased in a specific raffle*

## Migrate

