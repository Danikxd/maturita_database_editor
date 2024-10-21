use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "TV_channels")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub channel_name: String,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        panic!("No relations defined for tv_channels") 
    }
}

impl ActiveModelBehavior for ActiveModel {}