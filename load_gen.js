const net = require("net");


function load(){
    var client = new net.Socket();
    client.connect(8080, '127.0.0.1', function() {
      for(let i = 0; i < 500; i++){
        client.write('Hello, server!');
      }
      client.destroy();
    });
}

for (let i = 0; i < 240; i++){
  load();
}
