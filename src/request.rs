//! Type for making a generic request to the Matrix API.

use std::borrow::Cow;
use hyper::{Body, Method};
use std::collections::HashMap;
use serde::Serialize;
use serde::de::DeserializeOwned;
use hyper::client::Request;
use super::{MatrixFuture, MatrixClient};
use errors::MatrixResult;
use serde_json;
use percent_encoding::{utf8_percent_encode, DEFAULT_ENCODE_SET};
use futures;

/// A arbitrary request to an endpoint in the Matrix API.
///
/// This type has Super `Cow` Powers.
pub struct MatrixRequest<'a, T> {
    /// Request method (exported in the `http` module)
    pub meth: Method,
    /// API endpoint, without `/_matrix/client/r0` (e.g. `/sync`)
    pub endpoint: Cow<'a, str>,
    /// Query-string parameters.
    pub params: HashMap<Cow<'a, str>, Cow<'a, str>>,
    /// Request body (some type implementing `Serialize`).
    ///
    /// If this is empty (serialises to `{}`), it will not be sent. Therefore,
    /// requests with no body should use `()` here.
    pub body: T
}
impl<'a> MatrixRequest<'a, ()> {
    /// Convenience method for making a `MatrixRequest` from a method and
    /// endpoint.
    pub fn new_basic<S: Into<Cow<'a, str>>>(meth: Method, endpoint: S) -> Self {
        Self {
            meth,
            endpoint: endpoint.into(),
            params: HashMap::new(),
            body: ()
        }
    }
}
impl<'a, 'b, 'c> MatrixRequest<'a, HashMap<Cow<'b, str>, Cow<'c, str>>> {
    pub fn new_with_body<S, T, U, V>(meth: Method, endpoint: S, body: V) -> Self
        where S: Into<Cow<'a, str>>,
              T: Into<Cow<'b, str>>,
              U: Into<Cow<'c, str>>,
              V: IntoIterator<Item=(T, U)> {
        let body = body.into_iter().map(|(t, u)| (t.into(), u.into()))
            .collect();
        Self {
            meth,
            endpoint: endpoint.into(),
            params: HashMap::new(),
            body
        }
    }
}

impl<'a, T> MatrixRequest<'a, T> where T: Serialize {
    fn body(&self) -> MatrixResult<Option<Body>> {
        let body = serde_json::to_string(&self.body)?;
        Ok(if body == "{}" {
            None
        }
        else {
            Some(body.into())
        })
    }
    /// Makes a hyper `Request` from this type.
    ///
    /// The generated `Request` can then be sent to some unsuspecting Matrix
    /// homeserver using the `send_request()` or `send_discarding_request()`
    /// methods on `MatrixClient`.
    pub fn make_hyper(&self, client: &MatrixClient) -> MatrixResult<Request> {
        let body = self.body()?;
        let mut params = format!("access_token={}", client.access_token);
        for (k, v) in self.params.iter() {
            params += &format!("&{}={}",
                              utf8_percent_encode(k.as_ref(), DEFAULT_ENCODE_SET),
                              utf8_percent_encode(v.as_ref(), DEFAULT_ENCODE_SET));
        }
        let url = format!("{}/_matrix/client/r0{}?{}",
                          client.url,
                          self.endpoint,
                          params);
        let mut req = Request::new(self.meth.clone(), url.parse()?);
        if let Some(b) = body {
            req.set_body(b);
        }
        Ok(req)
    }
    /// Sends this request to a Matrix homeserver, expecting a deserializable
    /// `R` return type.
    ///
    /// A helpful mix of `make_hyper()` and `MatrixClient::send_request()`.
    pub fn send<R>(&self, mxc: &mut MatrixClient) -> MatrixFuture<R> where R: DeserializeOwned + 'static {
        let req = match self.make_hyper(mxc) {
            Ok(r) => r,
            Err(e) => return Box::new(futures::future::err(e.into()))
        };
        mxc.send_request(req)
    }
    /// Like `send()`, but uses `MatrixClient::send_discarding_request()`.
    pub fn discarding_send(&self, mxc: &mut MatrixClient) -> MatrixFuture<()> {
        let req = match self.make_hyper(mxc) {
            Ok(r) => r,
            Err(e) => return Box::new(futures::future::err(e.into()))
        };
        mxc.send_discarding_request(req)
    }
    // incredibly useful and relevant method
    pub fn moo() -> &'static str {
        r#"(__)
         (oo)
   /------\/
  / |    ||
 *  /\---/\
    ~~   ~~
....Cow::Borrowed("Have you mooed today?")..."#
    }
}
