const express = require('express');
const app = express();
app.get('/', (_req, res) => res.send('Express Fixture'));
module.exports = app;
