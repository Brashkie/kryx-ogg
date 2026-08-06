/**
 * Create npm sub-package directories with a proper package.json for each
 * platform @kryxjs/ogg supports.
 *
 * Runs automatically before `napi artifacts` via the "preartifacts" npm hook,
 * so the npm/<platform>/ folders exist when napi copies each .node into them.
 */

const fs = require('node:fs')
const path = require('node:path')

const mainPkg = JSON.parse(fs.readFileSync(path.join(__dirname, '..', 'package.json'), 'utf-8'))
const VERSION = mainPkg.version
const SCOPE = '@kryxjs'
const BASE_NAME = 'ogg'

// Each entry: folder name, os, cpu, optional libc, and the .node filename.
const PLATFORMS = [
  { folder: 'darwin-x64', os: 'darwin', cpu: 'x64', nodeFile: 'ogg.darwin-x64.node' },
  { folder: 'darwin-arm64', os: 'darwin', cpu: 'arm64', nodeFile: 'ogg.darwin-arm64.node' },
  { folder: 'win32-x64-msvc', os: 'win32', cpu: 'x64', nodeFile: 'ogg.win32-x64-msvc.node' },
  { folder: 'win32-arm64-msvc', os: 'win32', cpu: 'arm64', nodeFile: 'ogg.win32-arm64-msvc.node' },
  {
    folder: 'linux-x64-gnu',
    os: 'linux',
    cpu: 'x64',
    libc: 'glibc',
    nodeFile: 'ogg.linux-x64-gnu.node',
  },
  {
    folder: 'linux-x64-musl',
    os: 'linux',
    cpu: 'x64',
    libc: 'musl',
    nodeFile: 'ogg.linux-x64-musl.node',
  },
  {
    folder: 'linux-arm64-gnu',
    os: 'linux',
    cpu: 'arm64',
    libc: 'glibc',
    nodeFile: 'ogg.linux-arm64-gnu.node',
  },
]

const npmDir = path.join(__dirname, '..', 'npm')
if (!fs.existsSync(npmDir)) {
  fs.mkdirSync(npmDir, { recursive: true })
}

console.log('Creating npm sub-package directories...\n')

for (const platform of PLATFORMS) {
  const platformDir = path.join(npmDir, platform.folder)
  fs.mkdirSync(platformDir, { recursive: true })

  const subPkg = {
    name: `${SCOPE}/${BASE_NAME}-${platform.folder}`,
    version: VERSION,
    cpu: [platform.cpu],
    main: platform.nodeFile,
    files: [platform.nodeFile],
    description: `Native bindings for ${SCOPE}/${BASE_NAME} (${platform.os} ${platform.cpu})`,
    keywords: ['ogg', 'container', 'multimedia', 'audio', 'rust', 'napi'],
    author: mainPkg.author,
    license: mainPkg.license,
    homepage: mainPkg.homepage,
    repository: mainPkg.repository,
    bugs: mainPkg.bugs,
    engines: { node: '>= 18' },
    os: [platform.os],
  }

  if (platform.libc) {
    subPkg.libc = [platform.libc]
  }

  fs.writeFileSync(path.join(platformDir, 'package.json'), `${JSON.stringify(subPkg, null, 2)}\n`)

  const readme = `# \`${subPkg.name}\`

Platform-specific binary for [\`${SCOPE}/${BASE_NAME}\`](https://www.npmjs.com/package/${SCOPE}/${BASE_NAME}).

This is the **${platform.os} ${platform.cpu}${platform.libc ? ` (${platform.libc})` : ''}** build.

You should not install this package directly. Install \`${SCOPE}/${BASE_NAME}\` instead — npm installs this sub-package automatically based on your platform.

## License

${mainPkg.license}
`

  fs.writeFileSync(path.join(platformDir, 'README.md'), readme)
  console.log(`  ${subPkg.name}`)
}

console.log(`\nCreated ${PLATFORMS.length} platform sub-packages\n`)
