// Development web server redirecting requests to index.html
const express = require('express');
const morgan = require('morgan');

let app = express();
app.use(morgan('dev'));
app.use(express.static('dist'));
app.get('*', (req, res) => res.sendFile('index.html', { root: './dist' }));
app.listen(8080);