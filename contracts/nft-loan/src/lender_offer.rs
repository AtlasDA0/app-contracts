use cosmwasm_std::Addr;
use cw_storage_plus::{Index, IndexList, IndexedMap, MultiIndex};

use crate::state::OfferInfo;

pub struct LenderOfferIndexes<'a> {
    pub lender: MultiIndex<'a, Addr, OfferInfo, String>,
    pub borrower: MultiIndex<'a, Addr, OfferInfo, String>,
    pub loan: MultiIndex<'a, (Addr, u64), OfferInfo, String>,
}

impl<'a> IndexList<OfferInfo> for LenderOfferIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<OfferInfo>> + '_> {
        let v: Vec<&dyn Index<OfferInfo>> = vec![&self.lender, &self.borrower, &self.loan];
        Box::new(v.into_iter())
    }
}

pub fn lender_offers<'a>() -> IndexedMap<'a, &'a str, OfferInfo, LenderOfferIndexes<'a>> {
    let indexes = LenderOfferIndexes {
        lender: MultiIndex::new(
            |_, d: &OfferInfo| d.lender.clone(),
            "lender_offers",
            "lender_offers__lenderr",
        ),
        borrower: MultiIndex::new(
            |_, d: &OfferInfo| d.borrower.clone(),
            "lender_offers",
            "lender_offers__borrower",
        ),
        loan: MultiIndex::new(
            |_, d: &OfferInfo| (d.borrower.clone(), d.loan_id),
            "lender_offers",
            "lender_offers__collateral",
        ),
    };
    IndexedMap::new("lender_offers", indexes)
}
