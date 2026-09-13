import { expect, test } from 'bun:test'
import { readFile } from 'node:fs/promises'
import { resolve } from 'node:path'

const root = resolve(import.meta.dir, '..')

test('demo host does not require navigator.gpu and mentions WebGL2', async () => {
  const template = await readFile(resolve(root, 'demos/index.html'), 'utf8')
  expect(template).not.toContain('navigator.gpu')
  expect(template).toContain('WebGPU or WebGL2')
})

test('docs preview iframe allows WebGPU and WebGL embedding', async () => {
  const source = await readFile(
    resolve(root, 'src/components/docs/component-preview.tsx'),
    'utf8',
  )
  expect(source).toContain('allow="gpu; webgpu"')
  expect(source).toContain('WebGPU or WebGL2')
})
