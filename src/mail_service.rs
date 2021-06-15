use super::mail_db_types::*;
use super::utils::current_time_millis;
use mail_service_api::request;
use postgres::row::Row;
use postgres::GenericClient;

impl From<Row> for Mail {
  // select * from mail order only, otherwise it will fail
  fn from(row: Row) -> Mail {
    Mail {
      mail_id: row.get("mail_id"),
      request_id: row.get("request_id"),
      creation_time: row.get("creation_time"),
      topic: row.get("topic"),
      destination: row.get("destination"),
      title: row.get("title"),
      content: row.get("content"),
    }
  }
}

pub fn add(
  con: &mut impl GenericClient,
  props: request::MailNewProps,
) -> Result<Mail, postgres::Error> {
  let creation_time = current_time_millis();

  let mail_id = con
    .query_one(
      "INSERT INTO mail(
         request_id,
         creation_time,
         topic,
         destination,
         title,
         content
       )
       VALUES($1, $2, $3, $4, $5, $6, $7)
       RETURNING mail_id
      ",
      &[
        &props.request_id,
        &creation_time,
        &props.topic,
        &props.destination,
        &props.title,
        &props.content,
      ],
    )?
    .get(0);

  // return mail
  Ok(Mail {
    mail_id,
    request_id: props.request_id,
    creation_time,
    topic: props.topic,
    destination: props.destination,
    title: props.title,
    content: props.content,
  })
}

#[allow(unused)]
pub fn get_by_mail_id(
  con: &mut impl GenericClient,
  mail_id: i64,
) -> Result<Option<Mail>, postgres::Error> {
  let results = con
    .query_opt("SELECT * FROM mail WHERE mail_id=$1", &[&mail_id])?
    .map(|x| x.into());

  Ok(results)
}

pub fn query(
  con: &mut impl GenericClient,
  props: mail_service_api::request::MailViewProps,
) -> Result<Vec<Mail>, postgres::Error> {
  let results = con
    .query(
      "SELECT m.* FROM mail m WHERE 1 = 1
       AND ($1 == NULL OR m.mail_id = $1)
       AND ($2 == NULL OR m.request_id = $2)
       AND ($3 == NULL OR m.creation_time = $3)
       AND ($4 == NULL OR m.creation_time >= $4)
       AND ($5 == NULL OR m.creation_time <= $5)
       AND ($6 == NULL OR m.topic = $6)
       AND ($7 == NULL OR m.destination = $7)
       ORDER BY m.mail_id
       LIMIT $8, $9
      ",
      &[
        &props.mail_id,
        &props.request_id,
        &props.creation_time,
        &props.min_creation_time,
        &props.max_creation_time,
        &props.topic,
        &props.destination,
        &props.offset,
        &props.offset,
      ],
    )?
    .into_iter()
    .map(|row| row.into())
    .collect();

  Ok(results)
}
