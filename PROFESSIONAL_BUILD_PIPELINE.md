# RS2 Professional Build Pipeline

## New Commands

```bash
rs2 build
rs2 build --release
rs2 build --target windows
rs2 build --target linux
rs2 build --target web
rs2 publish --itch username/game
```

## Output Structure

build/
  windows/
    Demo.exe
    assets/
    scripts/
  linux/
  web/

## Pipeline Features
- Automatic engine workspace cache
- Automatic asset copy
- Compile validation
- Friendly errors
- Release packaging
- Cross-platform export

## Recommended Build Flow
1. Validate project structure
2. Copy engine source
3. Copy assets and scripts
4. Compile in release mode
5. Copy executable to final folder
6. Clean temporary cache
