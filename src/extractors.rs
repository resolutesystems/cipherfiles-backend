use std::error::Error;

use axum::async_trait;
use axum::extract::path::{ErrorKind, FailedToDeserializePathParams};
use axum::extract::rejection::{JsonRejection, PathRejection, QueryRejection};
use axum::extract::{FromRequest, FromRequestParts, Request};
use axum::http::request::Parts;
use num_ordinal::{ordinal0, Osize};
use serde::de::DeserializeOwned;

use crate::errors::AppError;

pub struct Json<T>(pub T);

#[async_trait]
impl<S, T> FromRequest<S> for Json<T>
where
    axum::Json<T>: FromRequest<S, Rejection = JsonRejection>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let (parts, body) = req.into_parts();
        let req = Request::from_parts(parts, body);

        match axum::Json::<T>::from_request(req, state).await {
            Ok(value) => Ok(Self(value.0)),
            Err(rejection) => {
                let str = match rejection {
                    JsonRejection::JsonDataError(error) => {
                        let s = match error.source() {
                            Some(src) => first_letter_uppercase(src.to_string()),
                            None => "".to_string()
                        };

                        format!("I couldn't deserialize JSON you sent me. {s}")
                    }
                    JsonRejection::JsonSyntaxError(_) => format!("There's an issue in JSON you sent me, plz fix it."),
                    JsonRejection::MissingJsonContentType(_) => format!("Hey.. Um.. It looks like you didn't send me correct content type! I need application/json."),
                    rej => rej.to_string(),
                };

                Err(AppError::Validation(str))
            }
        }
    }
}

pub struct Path<T>(pub T);

#[async_trait]
impl<S, T> FromRequestParts<S> for Path<T>
where
    T: DeserializeOwned + Send,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let path_parts = axum::extract::Path::<T>::from_request_parts(parts, state).await;
        match path_parts {
            Ok(value) => Ok(Self(value.0)),
            Err(rejection) => {
                let res = match rejection {
                    PathRejection::FailedToDeserializePathParams(inner) => {
                        handle_path_deserialize_rejection(inner)
                    }
                    PathRejection::MissingPathParams(error) => {
                        AppError::Validation(error.to_string())
                    }
                    err => {
                        tracing::warn!("Unhandled path rejection error: {err:?}");
                        AppError::Validation(String::from("unknown validation error."))
                    }
                };
                Err(res)
            }
        }
    }
}

pub struct Query<T>(pub T);

#[async_trait]
impl<S, T> FromRequestParts<S> for Query<T>
where
    T: DeserializeOwned + Send,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let path_parts = axum::extract::Query::<T>::from_request_parts(parts, state).await;
        match path_parts {
            Ok(value) => Ok(Self(value.0)),
            Err(rejection) => {
                let res = match rejection {
                    QueryRejection::FailedToDeserializeQueryString(f) => {
                        if let Some(source) = f.source() {
                            return Err(AppError::Validation(source.to_string()));
                        }
                        
                        AppError::Validation(String::from("missing query parameters."))
                    }
                    err => {
                        tracing::warn!("Unhandled query rejection error: {err:?}");
                        AppError::Validation(String::from("unknown validation error."))
                    }
                };
                Err(res)
            }
        }
    }
}

fn handle_path_deserialize_rejection(cause: FailedToDeserializePathParams) -> AppError {
    let kind = cause.into_kind();

    let str = match kind {
        ErrorKind::WrongNumberOfParameters { got, expected } => {
            format!("I need {expected} parameters for this endpoint! Not {got} which you gave me. Dummy.")
        }
        ErrorKind::ParseErrorAtIndex {
            index,
            value,
            expected_type,
        } => {
            let friendly_type = match expected_type {
                "i32" => "integer",
                "f32" => "decimal number",
                _ => expected_type,
            };
            let ord: Osize = ordinal0(index);
            let art = get_indefinite_article(friendly_type);

            format!("Hey! Your {ord} parameter, with '{value}' value, is not correct. It should be {art} {friendly_type}!")
        }
        ErrorKind::Message(msg) => {
            format!("Ouch, I couldn't validate this request parameters, because: {msg}")
        }
        err => {
            tracing::warn!("Path deserialize rejection error not implemented: {err:?}");
            err.to_string()
        }
    };

    AppError::Validation(str)
}

fn get_indefinite_article(word: &str) -> String {
    match word.to_lowercase().chars().next() {
        Some('a' | 'e' | 'i' | 'o' | 'u') => "an",
        _ => "a",
    }
    .to_string()
}

fn first_letter_uppercase(s: String) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().chain(c).collect(),
    }
}
