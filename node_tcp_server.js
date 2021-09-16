const net = require('net');


function listen(port){

const server = new net.Server();
server.listen(port, function() {
    console.log(`Server listening for connections on localhost:${port}`);
});

server.on('connection', function(socket) {
    //console.log('A new connection has been established.');
    //process.stdout.write("+");

    socket.on('data', function(chunk) {
        process.stdout.write(".");
    });

    // socket.on('end', function() {
    //     console.log('Closing connection with the client');
    // });

    socket.on('error', function(err) {
        console.log(`Error: ${err}`);
    });
});
}

process.argv.slice(2, process.argv.length).forEach(listen);
