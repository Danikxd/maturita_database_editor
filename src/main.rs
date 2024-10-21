mod model;

use model::series;
use model::tv_channels;

use sea_orm::TransactionTrait;
use serde::Deserialize;
use serde_xml_rs::from_str;
use std::env;
use std::fs::read_to_string;
use std::collections::HashMap;
use chrono::{DateTime, Utc};

use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, Database, DatabaseConnection, DatabaseTransaction,
    EntityTrait, QueryFilter, Set,
};

use sea_orm::{Condition, QueryOrder, sea_query::Expr};
use tv_channels::Entity as ChannelEntity;
use series::Entity as SeriesEntity;


use chrono::{NaiveTime, Timelike};

use dotenv::dotenv;

#[derive(Debug, Deserialize)]
struct TV {
    #[serde(rename = "channel", default)]
    channels: Vec<Channel>,
    #[serde(rename = "programme", default)]
    programmes: Vec<Programme>,
}

#[derive(Debug, Deserialize)]
struct Channel {
    #[serde(rename = "id")]
    id: String,
    #[serde(rename = "display-name")]
    display_name: String,
}

#[derive(Debug, Deserialize)]
struct Programme {
    #[serde(rename = "start", deserialize_with = "deserialize_datetime")]
    start: DateTime<Utc>,
    #[serde(rename = "stop", deserialize_with = "deserialize_datetime")]
    stop: DateTime<Utc>,

    #[serde(rename = "title")]
    title: String,
    #[serde(rename = "channel")]
    channel_id: String,
}

fn deserialize_datetime<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = String::deserialize(deserializer)?;
    // Parse the string into DateTime<Utc>
    let dt = chrono::DateTime::parse_from_str(&s, "%Y%m%d%H%M%S %z")
        .map_err(serde::de::Error::custom)?;
    Ok(dt.with_timezone(&Utc))
}

async fn get_channel_ids(
    db: &impl ConnectionTrait,
    xml_channels: &[Channel],
) -> Result<HashMap<String, i64>, sea_orm::DbErr> {
    let display_names: Vec<String> = xml_channels.iter().map(|c| c.display_name.clone()).collect();

    // Query the database for matching channels
    let channels = ChannelEntity::find()
        .filter(tv_channels::Column::ChannelName.is_in(display_names.clone()))
        .all(db)
        .await?;

    // Map channel names to their IDs
    let mut channel_name_to_id: HashMap<String, i64> = HashMap::new();
    for channel in channels {
        channel_name_to_id.insert(channel.channel_name.clone(), channel.id);
    }

    // Build a mapping from XML channel id to database channel id
    let mut xml_channel_to_db_id: HashMap<String, i64> = HashMap::new();
    for xml_channel in xml_channels {
        if let Some(&id) = channel_name_to_id.get(&xml_channel.display_name) {
            xml_channel_to_db_id.insert(xml_channel.id.clone(), id);
        } else {
            eprintln!(
                "Channel '{}' not found in the database.",
                xml_channel.display_name
            );
        }
    }

    Ok(xml_channel_to_db_id)
}

async fn update_programmes(
    db: &impl ConnectionTrait,
    programmes: &[Programme],
    channel_mapping: HashMap<String, i64>,
) -> Result<(), sea_orm::DbErr> {
    for programme in programmes {
        if let Some(&channel_id) = channel_mapping.get(&programme.channel_id) {
            
            let programme_start = programme.start.with_timezone(&Utc);
            let programme_end = programme.stop.with_timezone(&Utc);

            // Find an existing program by channel ID and start time
            let existing_programme = series::Entity::find()
                .filter(series::Column::ChannelId.eq(channel_id))
                .filter(series::Column::Start.eq(programme_start))
                .one(db)
                .await?;

            if let Some(existing) = existing_programme {
             
                if existing.title != programme.title || existing.end != programme_end {
                    let midnight = programme_start.date().and_hms(23, 59, 59);

                  
                    series::Entity::delete_many()
                        .filter(series::Column::ChannelId.eq(channel_id))
                        .filter(
                            Condition::all()
                                .add(series::Column::Start.gte(programme_start))
                                .add(series::Column::Start.lte(midnight)),
                        )
                        .exec(db)
                        .await?;

                
                  for p in programmes.iter().skip_while(|p| {
                    let p_start = p.start.with_timezone(&Utc);
                    Some(&channel_id) != channel_mapping.get(&p.channel_id) || p_start < programme_start
                }) {
                    if let Some(&id) = channel_mapping.get(&p.channel_id) {
                        let start_utc = p.start.with_timezone(&Utc);
                        if start_utc > midnight {
                            break; 
                        }

                     
                        let new_programme = series::ActiveModel {
                            channel_id: Set(id),
                            title: Set(p.title.clone()),
                            start: Set(start_utc),
                            end: Set(p.stop.with_timezone(&Utc)),
                            ..Default::default()
                        };
                        new_programme.insert(db).await?;
                    }
                }
                }
            } else {
                eprintln!(
                    "Channel ID '{}' not found in channel mapping.",
                    programme.channel_id
                );
            }
        } else {
            eprintln!(
                "Channel ID '{}' not found in channel mapping.",
                programme.channel_id
            );
        }
    }

    Ok(())
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read the XML data
    let xml_data = read_to_string("guide.xml")?;
    let tv: TV = from_str(&xml_data)?;

  
    dotenv().ok();

    
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    
    let db = Database::connect(&db_url).await?;

  
    let txn = db.begin().await?;

    
    let channel_mapping = get_channel_ids(&txn, &tv.channels).await?;

  
    update_programmes(&txn, &tv.programmes, channel_mapping).await?;

    
    txn.commit().await?;

    Ok(())
}
