use super::handlers;
use super::utils;
use super::Config;
use super::Db;
use super::SERVICE_NAME;
use mail_service_api::response::MailError;
use std::collections::HashMap;
use std::convert::Infallible;
use std::future::Future;
use warp::http::StatusCode;
use warp::Filter;

/// Helper to combine the multiple filters together with Filter::or, possibly boxing the types in
/// the process. This greatly helps the build times for `ipfs-http`.
/// https://github.com/seanmonstar/warp/issues/507#issuecomment-615974062
macro_rules! combine {
  ($x:expr, $($y:expr),+) => {{
      let filter = ($x).boxed();
      $( let filter = (filter.or($y)).boxed(); )+
      filter
  }}
}

/// The function that will show all ones to call
pub fn api(
    db: Db,
    config: Config,
) -> impl Filter<Extract = (impl warp::Reply,), Error = Infallible> + Clone {
    api_info()
        .or(combine!(
            adapter(
                config.clone(),
                db.clone(),
                warp::path!("mail_new"),
                handlers::mail_new,
            ),
            adapter(
                config.clone(),
                db.clone(),
                warp::path!("mail_view"),
                handlers::mail_view,
            )
        ))
        .recover(handle_rejection)
}

fn api_info() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    let mut info = HashMap::new();
    info.insert("version", "0.1");
    info.insert("name", SERVICE_NAME);
    warp::path!("public" / "info").map(move || warp::reply::json(&info))
}

// this function adapts a handler function to a warp filter
// it accepts an initial path filter
fn adapter<PropsType, ResponseType, F>(
    config: Config,
    db: Db,
    filter: impl Filter<Extract = (), Error = warp::Rejection> + Clone,
    handler: fn(Config, Db, PropsType) -> F,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone
where
    F: Future<Output = Result<ResponseType, MailError>> + Send,
    PropsType: Send + serde::de::DeserializeOwned,
    ResponseType: Send + serde::ser::Serialize,
{
    // lets you pass in an arbitrary parameter
    fn with<T: Clone + Send>(t: T) -> impl Filter<Extract = (T,), Error = Infallible> + Clone {
        warp::any().map(move || t.clone())
    }

    filter
        .and(with(config))
        .and(with(db))
        .and(warp::body::json())
        .and_then(move |config, db, props| async move {
            handler(config, db, props).await.map_err(mail_error)
        })
        .map(|x| warp::reply::json(&Ok::<ResponseType, ()>(x)))
}

// This function receives a `Rejection` and tries to return a custom
// value, otherwise simply passes the rejection along.
async fn handle_rejection(err: warp::Rejection) -> Result<impl warp::Reply, Infallible> {
    let code;
    let message;

    if err.is_not_found() {
        code = StatusCode::NOT_FOUND;
        message = MailError::NotFound;
    } else if err
        .find::<warp::filters::body::BodyDeserializeError>()
        .is_some()
    {
        code = StatusCode::BAD_REQUEST;
        message = MailError::BadRequest;
    } else if err.find::<warp::reject::MethodNotAllowed>().is_some() {
        code = StatusCode::METHOD_NOT_ALLOWED;
        message = MailError::MethodNotAllowed;
    } else if let Some(MailErrorRejection(mail_error)) = err.find() {
        code = StatusCode::BAD_REQUEST;
        message = mail_error.clone();
    } else {
        // We should have expected this... Just log and say its a 500
        utils::log(utils::Event {
            msg: "intercepted unknown error kind".to_owned(),
            source: None,
            severity: utils::SeverityKind::Error,
        });
        code = StatusCode::INTERNAL_SERVER_ERROR;
        message = MailError::Unknown;
    }

    Ok(warp::reply::with_status(
        warp::reply::json(&Err::<(), MailError>(message)),
        code,
    ))
}

// This type represents errors that we can generate
// These will be automatically converted to a proper string later
#[derive(Debug)]
pub struct MailErrorRejection(pub MailError);
impl warp::reject::Reject for MailErrorRejection {}

fn mail_error(mail_error: MailError) -> warp::reject::Rejection {
    warp::reject::custom(MailErrorRejection(mail_error))
}
