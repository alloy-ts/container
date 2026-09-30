const assert = require('node:assert/strict')
const test = require('node:test')

const { add } = require('./index.js')

test('adds two numbers', () => {
  assert.equal(add(2, 3), 5)
})
