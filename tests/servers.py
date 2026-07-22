import threading
from http.server import SimpleHTTPRequestHandler, HTTPServer

# Configuração dos servidores baseada no teu ficheiro TOML
servers = [
    {
        "port": 8080,
        "name": "landingpage",
        "html": """
        <!DOCTYPE html>
        <html lang="pt">
        <head>
            <meta charset="UTF-8">
            <title>Landing Page - Solitude</title>
            <style>
                body { font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif; text-align: center; margin-top: 60px; background-color: #f4f7f6; color: #333; }
                .container { max-width: 600px; margin: 0 auto; background: white; padding: 30px; border-radius: 10px; box-shadow: 0 4px 15px rgba(0,0,0,0.05); }
                h1 { color: #2c3e50; }
                p { font-size: 1.1em; line-height: 1.6; }
                .btn { display: inline-block; margin: 15px 10px; padding: 12px 25px; background-color: #3498db; color: white; text-decoration: none; border-radius: 5px; font-weight: bold; transition: background 0.2s; }
                .btn:hover { background-color: #2980b9; }
                .btn-alt { background-color: #2ecc71; }
                .btn-alt:hover { background-color: #27ae60; }
                .footer { margin-top: 30px; font-size: 0.9em; color: #7f8c8d; }
            </style>
        </head>
        <body>
            <div class="container">
                <h1>Bem-vindo à Landing Page!</h1>
                <p>Este servidor está a responder localmente na porta <strong>8080</strong> (<code>landingpage</code>).</p>
                <p>Usa os botões abaixo para testar o redirecionamento dinâmico efetuado pelo teu Proxy em Rust:</p>
                
                <a class="btn" href="http://teste1.mikaelmenezes.duckdns.org">Ir para o Teste 1 (Porta 8000)</a>
                <a class="btn btn-alt" href="http://teste2.mikaelmenezes.duckdns.org">Ir para o Teste 2 (Porta 8090)</a>
                
                <div class="footer">Configurado na placa Solitude</div>
            </div>
        </body>
        </html>
        """
    },
    {
        "port": 8000,
        "name": "teste1",
        "html": """
        <!DOCTYPE html>
        <html lang="pt">
        <head>
            <meta charset="UTF-8">
            <title>Servidor Teste 1</title>
            <style>
                body { font-family: sans-serif; text-align: center; margin-top: 80px; background-color: #ebf5fb; color: #2c3e50; }
                .box { display: inline-block; background: white; padding: 40px; border-radius: 8px; border-top: 5px solid #3498db; box-shadow: 0 4px 6px rgba(0,0,0,0.05); }
            </style>
        </head>
        <body>
            <div class="box">
                <h1>Servidor: Teste 1</h1>
                <p>Redirecionamento efetuado com sucesso para o subdomínio <code>teste1</code>!</p>
                <p>Este serviço está a correr na porta interna <strong>8000</strong>.</p>
            </div>
        </body>
        </html>
        """
    },
    {
        "port": 8090,
        "name": "teste2",
        "html": """
        <!DOCTYPE html>
        <html lang="pt">
        <head>
            <meta charset="UTF-8">
            <title>Servidor Teste 2</title>
            <style>
                body { font-family: sans-serif; text-align: center; margin-top: 80px; background-color: #e8f8f5; color: #16a085; }
                .box { display: inline-block; background: white; padding: 40px; border-radius: 8px; border-top: 5px solid #2ecc71; box-shadow: 0 4px 6px rgba(0,0,0,0.05); }
            </style>
        </head>
        <body>
            <div class="box">
                <h1>Servidor: Teste 2</h1>
                <p>Redirecionamento efetuado com sucesso para o subdomínio <code>teste2</code>!</p>
                <p>Este serviço está a correr na porta interna <strong>8090</strong>.</p>
            </div>
        </body>
        </html>
        """
    }
]

def make_handler(html_content):
    class DynamicHandler(SimpleHTTPRequestHandler):
        def do_GET(self):
            self.send_response(200)
            self.send_header("Content-type", "text/html; charset=utf-8")
            # Headers básicos de CORS se o teu frontend precisar de comunicar com a API
            self.send_header("Access-Control-Allow-Origin", "*")
            self.end_headers()
            self.wfile.write(html_content.encode('utf-8'))
        def log_message(self, format, *args):
            # Mantém o log limpo e focado no essencial
            pass
    return DynamicHandler

def start_server(port, name, html):
    try:
        server = HTTPServer(('127.0.0.1', port), make_handler(html))
        print(f"✅ Servidor [{name}] online em -> 127.0.0.1:{port}")
        server.serve_forever()
    except Exception as e:
        print(f"❌ Erro ao iniciar [{name}] na porta {port}: {e}")

if __name__ == "__main__":
    threads = []
    print("A iniciar servidores locais para testes do Proxy...")
    
    for s in servers:
        t = threading.Thread(target=start_server, args=(s["port"], s["name"], s["html"]), daemon=True)
        t.start()
        threads.append(t)
        
    try:
        for t in threads:
            t.join()
    except KeyboardInterrupt:
        print("\nA desligar os servidores de teste de forma segura...")
