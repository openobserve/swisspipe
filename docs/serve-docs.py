#!/usr/bin/env python3
"""
Simple HTTP server for serving SwissPipe API documentation.

Usage:
    python serve-docs.py [port]

Default port is 8080.
"""

import http.server
import socketserver
import webbrowser
import sys
import os
from pathlib import Path

def main():
    # Get port from command line argument, default to 8080
    port = 8080
    if len(sys.argv) > 1:
        try:
            port = int(sys.argv[1])
        except ValueError:
            print("Error: Port must be a valid integer")
            sys.exit(1)

    # Change to docs directory
    docs_dir = Path(__file__).parent
    os.chdir(docs_dir)

    # Create HTTP server
    handler = http.server.SimpleHTTPRequestHandler
    handler.extensions_map.update({
        '.yaml': 'text/yaml',
        '.yml': 'text/yaml',
    })

    with socketserver.TCPServer(("", port), handler) as httpd:
        print(f"🚀 SwissPipe API Documentation Server")
        print(f"📡 Serving at: http://localhost:{port}")
        print(f"📚 Documentation: http://localhost:{port}/api-documentation.html")
        print(f"📋 OpenAPI Spec: http://localhost:{port}/openapi.yaml")
        print(f"⏹️  Press Ctrl+C to stop the server")
        print()

        # Automatically open browser
        doc_url = f"http://localhost:{port}/api-documentation.html"
        print(f"🌐 Opening documentation in browser: {doc_url}")
        try:
            webbrowser.open(doc_url)
        except Exception as e:
            print(f"Could not open browser automatically: {e}")

        try:
            httpd.serve_forever()
        except KeyboardInterrupt:
            print(f"\n👋 Server stopped. Thanks for using SwissPipe API docs!")

if __name__ == "__main__":
    main()