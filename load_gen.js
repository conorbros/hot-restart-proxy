const net = require("net");


function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

function load(){
    var client = new net.Socket();
    client.connect(8080, '127.0.0.1', function() {
      for(let i = 0; i < 200; i++){
        client.write('Hello, server!');
        sleep(1000);
      }
      client.destroy();
    });
}

for (let i = 0; i < 200; i++){
  load();
}
