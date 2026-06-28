use async_trait::async_trait;
use pingora::{
    listeners::tls::TlsSettings,
    prelude::HttpPeer,
    proxy::{ProxyHttp, Session},
    server::configuration::Opt,
    server::Server,
};
use std::collections::HashMap;

// 1. Definição da estrutura do Proxy
struct Proxy {
    addrs: HashMap<String, String>,
}

// 2. O Contexto (bloco de notas) de cada requisição
pub struct Context {
    authenticated: bool,
}

impl Proxy {
    // Função auxiliar para forçar o pop-up de autenticação HTTP Basic no navegador
    async fn log(&self, session: &mut Session) -> pingora::Result<bool> {
        let mut response = pingora::http::ResponseHeader::build(401, None)?;
        response.insert_header("WWW-Authenticate", "Basic realm=\"Acesso Restrito\"")?;
        
        // Enviamos o cabeçalho e fechamos o stream (true), pois não há corpo HTML aqui
        session
            .write_response_header(Box::new(response), true)
            .await?;
        Ok(true)
    }
}

#[async_trait]
impl ProxyHttp for Proxy {
    type CTX = Context;

    // Inicializa o contexto para cada nova conexão
    fn new_ctx(&self) -> Self::CTX {
        Context {
            authenticated: false,
        }
    }

    // Camada de Segurança: Intercepta e valida as credenciais antes do roteamento
    async fn request_filter(
        &self,
        session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> pingora::Result<bool> {
        let auth = session.req_header().headers.get("Authorization");
        
        match auth {
            Some(value) => {
                let value = value.to_str().unwrap_or("");
                
                // Validação exata da sua string Base64 (mikael:mimadjka 4)
                if value == "Basic bWlrYWVsOm1pbWFkamthIDQ=" {
                    ctx.authenticated = true;
                    Ok(false) // Credenciais corretas! Prossegue para o upstream_peer
                } else {
                    self.log(session).await // Senha errada, exibe o pop-up novamente
                }
            }
            // Primeiro acesso (sem cabeçalho de autorização), solicita login
            None => self.log(session).await,
        }
    }

    // Camada de Roteamento: Encaminha a requisição autenticada para as portas internas
    async fn upstream_peer(
        &self,
        session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<Box<HttpPeer>> {
        let path = session.req_header().uri.path();
        
        // Define o destino baseado no começo do caminho (path) da URL
        let service="127.0.0.1:8000";
        // Cria o peer apontando para o localhost (HTTP puro internamente)
        let peer = HttpPeer::new(service, false, String::new());
        Ok(Box::new(peer))
    }
}

fn main() {
    // Inicialização da Engine do Servidor Pingora
    let opt = Opt::default();
    let mut server = Server::new(Some(opt)).unwrap();
    server.bootstrap();

    let meu_proxy = Proxy {
        addrs: HashMap::new(),
    };

    // Acopla a lógica do nosso proxy ao serviço HTTP do framework
    let mut proxy_service = pingora::proxy::http_proxy_service(
        &server.configuration,
        meu_proxy,
    );
    
    // Configuração pública do TLS/SSL utilizando os certificados do Let's Encrypt
let tls_settings = TlsSettings::intermediate(
        "/etc/letsencrypt/live/mikaelmenez15.duckdns.org/fullchain.pem",
        "/etc/letsencrypt/live/mikaelmenez15.duckdns.org/privkey.pem",
    ).expect("Falha ao carregar os certificados SSL");

    // 2. Vincula o proxy à porta física 443 com as chaves criptográficas ativas
    proxy_service.add_tls_with_settings("0.0.0.0:443", None, tls_settings);
    // Vincula o proxy à porta física 443 com as chaves criptográficas ativas

    // Adiciona o serviço estruturado ao servidor e inicia o loop eterno do daemon
    server.add_service(proxy_service);
    server.run_forever();
}

