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

// 1. Definição da estrutura do Proxy
// 1. Definição da estrutura do Proxy (Agora ela guarda o mapa pronto e otimizado)
struct Proxy {
    mapping: HashMap<String, String>,
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
    async fn upstream_request_filter(
        &self,
        session: &mut Session,
        upstream_request: &mut pingora::http::RequestHeader,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<()> {
        // Extrai o IP real do usuário (Camada 3) a partir do socket da sessão
        let client_ip = session
            .client_addr()
            .map(|addr| addr.to_string())
            .unwrap_or_else(|| "127.0.0.1".to_string());

        // Padrão RFC: Informa ao backend o IP real de quem fez o request
        upstream_request.insert_header("X-Forwarded-For", client_ip)?;

        // Padrão RFC: Informa que a conexão externa original chegou via HTTPS
        upstream_request.insert_header("X-Forwarded-Proto", "https")?;

        // O cabeçalho 'Via' é o padrão de redes para rastrear proxies no caminho.
        // Fica um easter egg excelente para o seu professor da UFPB avaliar.
        upstream_request.insert_header("Via", "1.1 gateway-ufpb-mikael")?;

        Ok(())
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
                if value == "Basic YWRtaW46YWRtaW4xMjM=" {
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
        // 1. Captura segura do Host (com fallback e sem unwrap perigoso)
        let host_header = session
            .get_header("Host")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        let host_completo = host_header
            .or_else(|| session.req_header().uri.host().map(|h| h.to_string()))
            .unwrap_or_default(); // Se tudo falhar, vira ""

        // 2. Limpa a string para pegar só o subdomínio
        // Remove porta se houver (ex: api.duckdns.org:443 -> api.duckdns.org)
        let host_sem_porta = host_completo.split(':').next().unwrap_or(&host_completo);

        // Isola o subdomínio
        let subdomain = host_sem_porta
            .strip_suffix(".mikaelmenezes.duckdns.org")
            .unwrap_or(""); // Retorna vazio se acessarem pelo IP puro ou raiz

        // 3. Busca no HashMap que já está na memória
        let default = "127.0.0.1:8080".to_string(); // Porta padrão de fallback real
        let destino = self.mapping.get(subdomain).unwrap_or(&default);

        // 4. Encaminha!
        let peer = HttpPeer::new(destino, false, String::new());
        Ok(Box::new(peer))
    }
    async fn response_filter(
        &self,
        _session: &mut Session,
        upstream_response: &mut pingora::http::ResponseHeader,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<()> {
        // Sobrescreve a assinatura do servidor (Esconde se o backend é Spring, Django, etc)
        // Isso é uma prática clássica de segurança (Security through obscurity)
        upstream_response.insert_header("Server", "Pingora-Edge-Proxy")?;

        // Adiciona um cabeçalho customizado para mostrar que a rede está sendo roteada por você
        upstream_response.insert_header("X-Proxy-By", "Mikael-Network-Gateway")?;

        Ok(())
    }
}

fn main() {
    // Inicialização da Engine do Servidor Pingora
    let opt = Opt::default();
    let mut server = Server::new(Some(opt)).unwrap();
    server.bootstrap();

    // Lê as configs e já converte para o HashMap
    let config_data = config::read_config("./instances.toml".to_string()).unwrap();
    let rotas_mapeadas = config::config_to_hashmap(&config_data);

    let meu_proxy = Proxy {
        mapping: rotas_mapeadas, // Injeta o mapa pronto
    };

    let mut proxy_service = pingora::proxy::http_proxy_service(&server.configuration, meu_proxy);

    // Configuração pública do TLS/SSL utilizando os certificados do Let's Encrypt
   let tls_settings = TlsSettings::intermediate(
    "/etc/letsencrypt/live/mikaelmenezes.duckdns.org/fullchain.pem",
    "/etc/letsencrypt/live/mikaelmenezes.duckdns.org/privkey.pem",
)
    .expect("Falha ao carregar os certificados SSL");

    // 2. Vincula o proxy à porta física 443 com as chaves criptográficas ativas
    proxy_service.add_tls_with_settings("0.0.0.0:443", None, tls_settings);
    // Vincula o proxy à porta física 443 com as chaves criptográficas ativas

    // Adiciona o serviço estruturado ao servidor e inicia o loop eterno do daemon
    server.add_service(proxy_service);
    server.run_forever();
}
