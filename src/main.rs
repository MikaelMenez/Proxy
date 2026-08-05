use async_trait::async_trait;
use pingora::{
    listeners::tls::TlsSettings,
    prelude::HttpPeer,
    proxy::{ProxyHttp, Session},
    server::Server,
    server::configuration::Opt,
};
use std::collections::HashMap;
mod config;


struct Proxy {
    mapping: HashMap<String, String>,
}

pub struct Context {
    authenticated: bool,
}

impl Proxy {
    async fn log(&self, session: &mut Session) -> pingora::Result<bool> {
        let mut response = pingora::http::ResponseHeader::build(401, None)?;
        response.insert_header("WWW-Authenticate", "Basic realm=\"Acesso Restrito\"")?;

        session
            .write_response_header(Box::new(response), true)
            .await?;
        Ok(true)
    }
}

#[async_trait]
impl ProxyHttp for Proxy {
    type CTX = Context;

    fn new_ctx(&self) -> Self::CTX {
        Context {
            authenticated: false,
        }
    }
    async fn upstream_request_filter(
        &self,
        session: &mut Session,
        upstream_request: &mut pingora::http::RequestHeader,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<()> {
        let client_ip = session
            .client_addr()
            .map(|addr| addr.to_string())
            .unwrap_or_else(|| "127.0.0.1".to_string());

        upstream_request.insert_header("X-Forwarded-For", client_ip)?;

        upstream_request.insert_header("X-Forwarded-Proto", "https")?;

        
        upstream_request.insert_header("Via", "1.1 gateway-ufpb-mikael")?;

        Ok(())
    }
    async fn request_filter(
        &self,
        session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> pingora::Result<bool> {
        let auth = session.req_header().headers.get("Authorization");

        match auth {
            Some(value) => {
                let value = value.to_str().unwrap_or("");

                if value == "Basic YWRtaW46YWRtaW4xMjM=" {
                    ctx.authenticated = true;
                    Ok(false) 
                } else {
                    self.log(session).await 
                }
            }
            None => self.log(session).await,
        }
    }

    async fn upstream_peer(
        &self,
        session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<Box<HttpPeer>> {
        let host_header = session
            .get_header("Host")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        let host_completo = host_header
            .or_else(|| session.req_header().uri.host().map(|h| h.to_string()))
            .unwrap_or_default(); 

        let host_sem_porta = host_completo.split(':').next().unwrap_or(&host_completo);

        let subdomain = host_sem_porta
            .strip_suffix(".mikaelmenezes.duckdns.org")
            .unwrap_or(""); 
        let default = "127.0.0.1:8080".to_string(); 
        let destino = self.mapping.get(subdomain).unwrap_or(&default);

        let peer = HttpPeer::new(destino, false, String::new());
        Ok(Box::new(peer))
    }
    async fn response_filter(
        &self,
        _session: &mut Session,
        upstream_response: &mut pingora::http::ResponseHeader,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<()> {
        upstream_response.insert_header("Server", "Pingora-Edge-Proxy")?;

        upstream_response.insert_header("X-Proxy-By", "Mikael-Network-Gateway")?;

        Ok(())
    }
}

fn main() {
    let opt = Opt::default();
    let mut server = Server::new(Some(opt)).unwrap();
    server.bootstrap();

    let config_data = config::read_config("./instances.toml".to_string()).unwrap();
    let rotas_mapeadas = config::config_to_hashmap(&config_data);

    let meu_proxy = Proxy {
        mapping: rotas_mapeadas, 
    };

    let mut proxy_service = pingora::proxy::http_proxy_service(&server.configuration, meu_proxy);

   let tls_settings = TlsSettings::intermediate(
    "/etc/letsencrypt/live/mikaelmenezes.duckdns.org/fullchain.pem",
    "/etc/letsencrypt/live/mikaelmenezes.duckdns.org/privkey.pem",
)
    .expect("Falha ao carregar os certificados SSL");

    proxy_service.add_tls_with_settings("0.0.0.0:443", None, tls_settings);

    server.add_service(proxy_service);
    server.run_forever();
}
