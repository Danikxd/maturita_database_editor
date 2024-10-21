use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "Series")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub channel_id: i64,
    pub title: String,
    pub start: DateTimeUtc,
    pub end: DateTimeUtc
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {}


impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        panic!("No Relation Defined")
    }
}

impl ActiveModelBehavior for ActiveModel {}