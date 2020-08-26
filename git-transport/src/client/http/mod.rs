use crate::{client, client::git, Protocol, Service};
use std::{
    borrow::Cow,
    convert::Infallible,
    io::{BufRead, Read},
};

#[cfg(feature = "http-client-curl")]
pub(crate) mod curl;

mod traits;
use crate::client::RequestWriter;
use git_packetline::PacketLine;
pub use traits::{Error, GetResponse, Http, PostResponse};

#[cfg(feature = "http-client-curl")]
pub type Impl = curl::Curl;

pub struct Transport<H: Http> {
    url: String,
    user_agent_header: &'static str,
    version: crate::Protocol,
    http: H,
    service: Option<Service>,
    line_provider: Option<git_packetline::Provider<H::ResponseBody>>,
}

impl Transport<Impl> {
    pub fn new(url: &str, version: crate::Protocol) -> Self {
        Transport {
            url: url.to_owned(),
            user_agent_header: concat!("User-Agent: git/oxide-", env!("CARGO_PKG_VERSION")),
            version,
            service: None,
            http: Impl::default(),
            line_provider: None,
        }
    }
}

impl<H: Http> Transport<H> {
    fn check_content_type(service: Service, kind: &str, headers: <H as Http>::Headers) -> Result<(), client::Error> {
        let wanted_content_type = format!("Content-Type: application/x-{}-{}", service.as_str(), kind);
        if !headers
            .lines()
            .collect::<Result<Vec<_>, _>>()?
            .iter()
            .any(|l| l == &wanted_content_type)
        {
            return Err(client::Error::Http(Error::Detail(format!(
                "Didn't find '{}' header to indicate 'smart' protocol, and 'dumb' protocol is not supported.",
                wanted_content_type
            ))));
        }
        Ok(())
    }
}

fn append_url(base: &str, suffix: &str) -> String {
    if base.ends_with('/') {
        format!("{}{}", base, suffix)
    } else {
        format!("{}/{}", base, suffix)
    }
}

impl<H: Http> client::Transport for Transport<H> {
    fn handshake(&mut self, service: Service) -> Result<client::SetServiceResponse, client::Error> {
        let url = append_url(&self.url, &format!("info/refs?service={}", service.as_str()));
        let static_headers = [Cow::Borrowed(self.user_agent_header)];
        let mut dynamic_headers = Vec::<Cow<str>>::new();
        if self.version != Protocol::V1 {
            dynamic_headers.push(Cow::Owned(format!("Git-Protocol: version={}", self.version as usize)));
        }
        let GetResponse { headers, body } = self.http.get(&url, static_headers.iter().chain(&dynamic_headers))?;
        <Transport<H>>::check_content_type(service, "advertisement", headers)?;

        let line_reader = self
            .line_provider
            .get_or_insert_with(|| git_packetline::Provider::new(body, PacketLine::Flush));

        let mut announced_service = String::new();
        line_reader.as_read().read_to_string(&mut announced_service)?;
        let expected_service_announcement = format!("# service={}", service.as_str());
        if announced_service.trim() != expected_service_announcement {
            return Err(client::Error::Http(Error::Detail(format!(
                "Expected to see {:?}, but got {:?}",
                expected_service_announcement,
                announced_service.trim()
            ))));
        }

        let (capabilities, refs) = git::recv::capabilties_and_possibly_refs(line_reader, self.version)?;
        self.service = Some(service);
        Ok(client::SetServiceResponse {
            actual_protocol: self.version,
            capabilities,
            refs,
        })
    }

    fn request(
        &mut self,
        write_mode: client::WriteMode,
        on_drop: Vec<client::MessageKind>,
        handle_progress: Option<client::HandleProgress>,
    ) -> Result<client::RequestWriter, client::Error> {
        let service = self.service.expect("handshake() must have been called first");
        let url = append_url(&self.url, service.as_str());
        let headers = &[
            format!("Content-Type: application/x-git-{}-request", service.as_str()),
            format!("Accept: application/x-git-{}-result", service.as_str()),
            "Expect:".into(),
        ];
        let PostResponse {
            headers,
            body,
            post_body,
        } = self.http.post(&url, headers)?;
        // <Transport<H>>::check_content_type(service, "result", headers)?;
        let line_provider = self
            .line_provider
            .as_mut()
            .expect("handshake to have been called first");
        line_provider.replace(body);
        Ok(RequestWriter::new(
            post_body,
            line_provider,
            write_mode,
            on_drop,
            handle_progress,
        ))
    }
}

pub fn connect(url: &str, version: crate::Protocol) -> Result<Transport<Impl>, Infallible> {
    Ok(Transport::new(url, version))
}
