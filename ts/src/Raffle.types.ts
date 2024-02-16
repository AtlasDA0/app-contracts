/**
* This file was automatically generated by @cosmwasm/ts-codegen@0.25.2.
* DO NOT MODIFY IT BY HAND. Instead, modify the source JSONSchema file,
* and run the @cosmwasm/ts-codegen generate command to regenerate this file.
*/

export type ExecuteMsg = {
  create_raffle: {
    assets: AssetInfo[];
    autocycle?: boolean | null;
    owner?: string | null;
    raffle_options: RaffleOptionsMsg;
    raffle_ticket_price: AssetInfo;
  };
} | {
  cancel_raffle: {
    raffle_id: number;
  };
} | {
  update_config: {
    creation_coins?: Coin[] | null;
    fee_addr?: string | null;
    minimum_raffle_duration?: number | null;
    minimum_raffle_timeout?: number | null;
    name?: string | null;
    nois_proxy_addr?: string | null;
    nois_proxy_coin?: Coin | null;
    owner?: string | null;
    raffle_fee?: Decimal | null;
  };
} | {
  modify_raffle: {
    raffle_id: number;
    raffle_options: RaffleOptionsMsg;
    raffle_ticket_price?: AssetInfo | null;
  };
} | {
  buy_ticket: {
    raffle_id: number;
    sent_assets: AssetInfo;
    ticket_count: number;
  };
} | {
  receive: Cw721ReceiveMsg;
} | {
  determine_winner: {
    raffle_id: number;
  };
} | {
  nois_receive: {
    callback: NoisCallback;
  };
} | {
  toggle_lock: {
    lock: boolean;
  };
} | {
  update_randomness: {
    raffle_id: number;
  };
};
export type AssetInfo = {
  cw721_coin: Cw721Coin;
} | {
  coin: Coin;
} | {
  sg721_token: Sg721Token;
};
export type Uint128 = string;
export type Timestamp = Uint64;
export type Uint64 = string;
export type Decimal = string;
export type Binary = string;
export type HexBinary = string;
export interface Cw721Coin {
  address: string;
  token_id: string;
}
export interface Coin {
  amount: Uint128;
  denom: string;
  [k: string]: unknown;
}
export interface Sg721Token {
  address: string;
  token_id: string;
}
export interface RaffleOptionsMsg {
  comment?: string | null;
  max_ticket_number?: number | null;
  max_ticket_per_address?: number | null;
  raffle_duration?: number | null;
  raffle_preview?: number | null;
  raffle_start_timestamp?: Timestamp | null;
  raffle_timeout?: number | null;
}
export interface Cw721ReceiveMsg {
  msg: Binary;
  sender: string;
  token_id: string;
}
export interface NoisCallback {
  job_id: string;
  published: Timestamp;
  randomness: HexBinary;
}
export interface InstantiateMsg {
  creation_coins?: Coin[] | null;
  fee_addr?: string | null;
  max_ticket_number?: number | null;
  minimum_raffle_duration?: number | null;
  minimum_raffle_timeout?: number | null;
  name: string;
  nois_proxy_addr: string;
  nois_proxy_coin: Coin;
  owner?: string | null;
  raffle_fee: Decimal;
}
export type QueryMsg = {
  config: {};
} | {
  raffle_info: {
    raffle_id: number;
  };
} | {
  all_raffles: {
    filters?: QueryFilters | null;
    limit?: number | null;
    start_after?: number | null;
  };
} | {
  all_tickets: {
    limit?: number | null;
    raffle_id: number;
    start_after?: number | null;
  };
} | {
  ticket_count: {
    owner: string;
    raffle_id: number;
  };
};
export interface QueryFilters {
  contains_token?: string | null;
  owner?: string | null;
  states?: string[] | null;
  ticket_depositor?: string | null;
}
export type Addr = string;
export type RaffleState = "created" | "started" | "closed" | "finished" | "claimed" | "cancelled";
export interface AllRafflesResponse {
  raffles: RaffleResponse[];
}
export interface RaffleResponse {
  raffle_id: number;
  raffle_info?: RaffleInfo | null;
  raffle_state: RaffleState;
}
export interface RaffleInfo {
  assets: AssetInfo[];
  is_cancelled: boolean;
  number_of_tickets: number;
  owner: Addr;
  raffle_options: RaffleOptions;
  raffle_ticket_price: AssetInfo;
  randomness?: HexBinary | null;
  winner?: Addr | null;
}
export interface RaffleOptions {
  comment?: string | null;
  max_ticket_number?: number | null;
  max_ticket_per_address?: number | null;
  raffle_duration: number;
  raffle_preview: number;
  raffle_start_timestamp: Timestamp;
  raffle_timeout: number;
}
export type ArrayOfString = string[];
export interface ConfigResponse {
  creation_coins: Coin[];
  fee_addr: Addr;
  last_raffle_id: number;
  lock: boolean;
  minimum_raffle_duration: number;
  minimum_raffle_timeout: number;
  name: string;
  nois_proxy_addr: Addr;
  nois_proxy_coin: Coin;
  owner: Addr;
  raffle_fee: Decimal;
}
export type Uint32 = number;