use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
pub enum CardVisibility {
    #[sqlx(rename = "private")]
    Private,
    #[sqlx(rename = "restricted")]
    Restricted,
    #[sqlx(rename = "public")]
    Public,
}

impl std::fmt::Display for CardVisibility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CardVisibility::Private => write!(f, "private"),
            CardVisibility::Restricted => write!(f, "restricted"),
            CardVisibility::Public => write!(f, "public"),
        }
    }
}

impl std::str::FromStr for CardVisibility {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "private" => Ok(CardVisibility::Private),
            "restricted" => Ok(CardVisibility::Restricted),
            "public" => Ok(CardVisibility::Public),
            _ => Err(format!("Invalid visibility: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Card {
    pub id: Uuid,
    pub column_id: Uuid,
    pub title: String,
    pub body: Option<String>,
    pub position: i32,
    pub visibility: String,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub due_date: Option<NaiveDate>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCard {
    pub title: String,
    pub body: Option<String>,
    pub position: Option<i32>,
    pub visibility: Option<CardVisibility>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub due_date: Option<NaiveDate>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCard {
    pub title: Option<String>,
    pub body: Option<String>,
    pub visibility: Option<CardVisibility>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub due_date: Option<NaiveDate>,
}

#[derive(Debug, Deserialize)]
pub struct MoveCard {
    pub column_id: Uuid,
    pub position: i32,
}

#[derive(Debug, Deserialize, Default)]
pub struct CardFilter {
    pub tags: Option<Vec<Uuid>>,
    pub query: Option<String>,
    pub start_date_from: Option<NaiveDate>,
    pub start_date_to: Option<NaiveDate>,
    pub end_date_from: Option<NaiveDate>,
    pub end_date_to: Option<NaiveDate>,
    pub due_date_from: Option<NaiveDate>,
    pub due_date_to: Option<NaiveDate>,
    pub updated_from: Option<DateTime<Utc>>,
    pub updated_to: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct CardResponse {
    pub id: Uuid,
    pub column_id: Uuid,
    pub title: String,
    pub body: Option<String>,
    pub position: i32,
    pub visibility: String,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub due_date: Option<NaiveDate>,
    pub tags: Vec<super::tag::TagResponse>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Card {
    pub fn into_response(self, tags: Vec<super::tag::TagResponse>) -> CardResponse {
        CardResponse {
            id: self.id,
            column_id: self.column_id,
            title: self.title,
            body: self.body,
            position: self.position,
            visibility: self.visibility,
            start_date: self.start_date,
            end_date: self.end_date,
            due_date: self.due_date,
            tags,
            created_by: self.created_by,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}
