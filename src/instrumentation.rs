use std::{io::IsTerminal, time::Duration};

use axum::{
    http::{Request, Response},
    Router,
};
use tower_http::trace::TraceLayer;
use tracing::Span;
use tracing_error::ErrorLayer;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::utilities::friendly_id;

pub fn setup(directives: &[String]) -> anyhow::Result<()> {
    let filter = filter_layer(directives)?;

    tracing_subscriber::registry()
        .with(filter)
        .with(ErrorLayer::default())
        .with(
            fmt::Layer::new()
                .with_ansi(std::io::stderr().is_terminal())
                .with_writer(std::io::stderr)
                .compact()
                .without_time()
                .with_target(false)
                .with_thread_ids(false)
                .with_thread_names(false)
                .with_file(false)
                .with_line_number(false),
        )
        .init();

    Ok(())
}

fn filter_layer(directives: &[String]) -> anyhow::Result<EnvFilter> {
    let mut layer = EnvFilter::default();

    for directive in directives {
        layer = layer.add_directive(directive.parse()?);
    }

    Ok(layer)
}

pub fn add_layer(router: Router) -> Router {
    router.layer(
        TraceLayer::new_for_http()
            .make_span_with(|req: &Request<_>| {
                tracing::span!(
                    tracing::Level::INFO,
                    "request",
                    id = %friendly_id(8),
                    uri = %req.uri(),
                    method = %req.method(),
                    status = tracing::field::Empty,
                    latency = tracing::field::Empty,
                )
            })
            .on_request(|_: &Request<_>, _: &Span| {
                tracing::trace!("got request");
            })
            .on_response(|res: &Response<_>, latency: Duration, span: &Span| {
                span.record(
                    "latency",
                    tracing::field::display(format!("{}ms", latency.as_millis())),
                );
                span.record("status", tracing::field::display(res.status()));
                tracing::trace!("responded");
            }),
    )
}
