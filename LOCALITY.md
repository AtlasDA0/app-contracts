# Locality 

Localities are raffles that randomly selects winners at scheduled intervals. This contract makes use of the `x/cron` module, which allows us to run logic during every block.

## TODO:
- fix addr retrieval for winner/minting 
- route fees, add fee-destination to ticket purchase
- integration tests

### Creating New Locality 

To create a new locality, you will need to: 
1. Define the new collection metadata via `CollectionParams`. 
2. Define your desired locality instance configuration via `LocalityMinterParams`

#### `LocalityMinterParams`
- `start_time` - the time trading will be available 
- `num_tokens` -  number of tokens of the new collection that will be minted
- `mint_price` - price of token(ticket) for minting
- `max_tickets` - number of ticket able for purchase
- `per_address_limit` -  number of tickets an address can purchase
- `duration` - length in seconds of the minting timeframe
- `frequency` - frequency in blocks for the minting of new tokens
- `harmonics` -  number of winners determined in phase alignment

#### Phase Alignment 
Phase alignment is a term used inside these contract for whether or not the interval set by locality creators is *in phase* with the current block height the contracts are deployed on. 

### Minting A Locality Token
In order to mint a locality token, you must have purchased one of the tickets that are randomly chosen by the contract, during the intervals *(`frequency`)* defined by the creator of the locality instance. 

