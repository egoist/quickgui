import { expect, test } from 'bun:test'
import { readFile } from 'node:fs/promises'
import { resolve } from 'node:path'

test('hashed Vite assets are cached permanently', async () => {
  const headers = await readFile(resolve(import.meta.dir, '../public/_headers'), 'utf8')
  expect(headers).toMatch(
    /\/assets\/\*\n[ \t]+Cache-Control: public, max-age=31536000, immutable/,
  )
})

test('demo pages advertise WebGPU permission policy', async () => {
  const headers = await readFile(resolve(import.meta.dir, '../public/_headers'), 'utf8')
  expect(headers).toMatch(/\/demos\/\*\n[ \t]+Permissions-Policy: gpu=\(self\)/)
})
