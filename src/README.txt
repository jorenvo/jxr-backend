To run server:

$ cargo run

This will run the server at 127.0.0.1:8000. To allow CORS proxy requests through http-server:

$ npx http-server --cors --proxy 'http://127.0.0.1:8000'

And connect the client to port 8081 (default of http-server).

Check cached pages:

$ vmtouch -i '.git' -i '*.po' -i '*.pot' .

Cache pages:

$ vmtouch -t -i '.git' -i '*.po' -i '*.pot' .