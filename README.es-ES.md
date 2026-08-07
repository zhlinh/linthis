
</think>

# linthis

[![Crates.io](https://img.shields.io/crates/v/linthis.svg)](https://crates.io/crates/linthis)
[![PyPI](https://img.shields.io/pypi/v/linthis.svg)](https://pypi.org/project/linthis/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Un linter y formateador rápido, multiplataforma y multiidioma escrito en Rust.

## Características

- 🚀 **Un solo comando**: Ejecuta comprobaciones de lint, formateo, seguridad y complejidad simultáneamente
- 🌍 **Soporte multiidioma**: Rust, Python, TypeScript, JavaScript, Go, Java, C++, Swift, Kotlin, Lua y más
- 🎯 **Detección automática**: Detecta automáticamente los lenguajes de programación utilizados en tu proyecto
- 🔒 **Escaneo de seguridad (SAST)**: Detección integrada de secretos + integración con OpenGrep/Semgrep, Bandit, Gosec, Flawfinder
- 📊 **Análisis de complejidad**: Complejidad ciclomática/cognitiva por función con aplicación de umbrales
- ⚙️ **Configuración flexible**: Soporte para configuración de proyecto, configuración global y parámetros CLI con rutas de claves (dotted key paths)
- 📦 **Sistema de plugins**: Comparte y reutiliza configuraciones mediante repositorios Git (HTTPS con fallback automático a SSH)
- 🎨 **Preajustes de formato**: Soporte para estilos de código populares como Google, Airbnb, Standard
- ⚡ **Procesamiento en paralelo**: Aprovecha CPUs de múltiples núcleos para un procesamiento de archivos más rápido con caché por archivo
- 🤖 **Revisión de código con IA**: `linthis review` analiza diffs con IA y crea PR/MR automáticamente
- 💾 **Copia de seguridad y deshacer**: `linthis backup undo` / `linthis backup redo` para restaurar cambios, con archivos de parche de git para revisión
- 🔄 **Modo de corrección de commits**: Tres modos para manejar correcciones automáticas — `squash` (fusionar en el commit), `dirty` (dejar para revisión), `fixup` (commit separado)
- 🔌 **Agrupación de ganchos de plugins**: Los plugins pueden incluir scripts de ganchos git y de agentes personalizados, instalados automáticamente al añadir el plugin
- 🆙 **Autoactualización**: `linthis update` con detección del método de instalación (cargo, pip, uv, pipx)
- 📝 **Auto .gitignore**: Genera .gitignore cuando falta, bloquea commits con archivos que deberían ignorarse
- 🗂️ **Almacenamiento global de datos**: Los datos en tiempo de ejecución (resultados, copias de seguridad, caché) se almacenan en `~/.linthis/projects/` — mantiene limpia la carpeta del proyecto

## Instalación

### Método 1: Instalar vía PyPI (Recomendado)

```bash
# Usando uv (recomendado)
pip install uv
uv tool install linthis

# Usando pip
pip install linthis
```

### Método 2: Instalar vía Homebrew (Recomendado para usuarios de macOS/Linux)

```bash
brew tap zhlinh/linthis
brew install linthis
```

### Método 3: Instalar vía Cargo (Recomendado para usuarios de Rust)

```bash
cargo install linthis
```

### Método 4: Compilar desde el código fuente

```bash
git clone https://github.com/zhlinh/linthis.git
cd linthis
cargo build --release
```

## Inicio rápido

Instala linthis, añade un plugin, configura los ganchos y ejecuta tu primera comprobación — todo en menos de un minuto.

```bash
# 1. Instalar
uv tool install linthis

# 2. Añadir plugin de equipo (-g es ámbito de usuario, usa la URL del plugin de tu equipo)
linthis plugin add -g sample https://github.com/zhlinh/linthis-plugin-template

# 3. Instalar ganchos (-g es ámbito de usuario)
linthis hook install -g                                           # gancho pre-commit de git
linthis hook install -g --type git-with-agent --provider claude  # gancho git + auto-reparación con IA en caso de fallo
linthis hook install -g --type agent --provider claude            # gancho de agente de IA (Claude, Cursor, etc.)

# 4. Ejecutar comprobación de lint
linthis -i src/

# 5. Comprobar archivos en stage antes del commit
git add src/main.py
linthis -s
```

<video src="docs/assets/videos/QuickStart-en.mp4" controls width="100%"></video>

> Consulta más tutoriales en vídeo en la página [Tutoriales en vídeo](docs/getting-started/videos.md).

### Inicializar configuración (Opcional)

```bash
# Crear archivo de configuración del proyecto
linthis init

# Crear archivo de configuración global
linthis init -g

# Ganchos a nivel de proyecto
linthis hook install                                          # gancho pre-commit de git
linthis hook install --type git-with-agent --provider claude  # gancho git + auto-reparación con IA en caso de fallo
linthis hook install --type agent --provider claude           # reglas del agente de IA (Claude Code)
linthis hook install --type prek                              # gancho pre-commit de prek
linthis hook install --event pre-push                         # gancho pre-push de git
linthis hook install --event commit-msg                       # gancho de formato de mensaje de commit

# Ganchos globales (se aplican a todos los repos de esta máquina)
linthis hook install --global                                 # pre-commit global de git
linthis hook install --global --type git-with-agent --provider claude  # global + auto-reparación con IA
linthis hook install --type agent --provider claude --global  # reglas del agente de IA (directorio home del usuario)
linthis hook install --global --event commit-msg              # gancho de formato de mensaje de commit global

# Forzar sobrescritura de archivos existentes
linthis init --force
linthis hook install --force
```

### Omitir ganchos selectivamente

`git commit --no-verify` desactiva todos los ganchos a la vez. Usa estas variables de entorno para omitir solo lo que necesites:

```bash
# Omitir un gancho específico. Tokens soportados (separados por comas, sin distinguir mayúsculas/minúsculas):
#   check | pc        →  pre-commit + post-commit (el par)
#   pre-commit        →  solo pre-commit
#   post-commit       →  solo post-commit
#   cmsg | cm         →  commit-msg
#   pp                →  pre-push
#   all               →  todo
LINTHIS_SKIP=cm       git commit -m "temp: omitir regex de commit-msg"
LINTHIS_SKIP=pc       git commit -m "WIP: omitir lint/seguridad/complejidad"
LINTHIS_SKIP=cm,pc    git commit -m "omitir ambos"

# Omitir comprobaciones específicas dentro de pre-commit (funciona con prefijos de ≥3 caracteres):
#   lint | lin        →  omitir lint
#   security | sec    →  omitir seguridad/SAST
#   complexity | com  →  omitir complejidad
LINTHIS_SKIP_CHECKS=com  git commit -m "fix: omitir complejidad lenta"
LINTHIS_SKIP_CHECKS=lin,com  git commit -m "fix: solo ejecutar seguridad"
```

### Uso básico

```bash
# Comprobar y formatear el directorio actual (comportamiento por defecto)
linthis

# Comprobar y formatear directorios específicos
linthis -i src/
linthis --include src/ --include lib/

# Solo comprobación, sin formateo
linthis -c
linthis --check-only

# Solo formateo, sin comprobación
linthis -f
linthis --format-only

# Comprobar archivos en stage de Git (adecuado para ganchos pre-commit)
linthis -s
linthis --staged

# Comprobar todos los archivos modificados localmente (en stage + sin stage)
linthis -m
linthis --modified

# Ejecutar comprobaciones específicas (por defecto: lint + security + complexity)
linthis --checks lint,security     # Solo lint + security
linthis --checks all               # Todas las comprobaciones
linthis --checks lint              # Solo lint (sin security/complexity)
```

### Especificar lenguajes

```bash
# Comprobar un lenguaje específico
linthis -l python
linthis --lang rust

# Comprobar múltiples lenguajes
linthis -l python,rust,cpp
linthis --lang "python,javascript,go"
```

### Excluir archivos

```bash
# Excluir patrones específicos
linthis -e "*.test.js" -e "dist/**"
linthis --exclude "target/**" --exclude "node_modules/**"
```

## Sistema de plugins

linthis admite plugins de configuración basados en Git para compartir fácilmente estándares de código entre proyectos y equipos.

<video src="docs/assets/videos/PluginSystem-en.mp4" controls width="100%"></video>

### Añadir plugin

```bash
# Añadir plugin a la configuración del proyecto (.linthis/config.toml)
linthis plugin add <alias> <git-url>

# Ejemplo: Añadir un plugin personalizado
linthis plugin add myplugin https://github.com/zhlinh/linthis-plugin.git

# Añadir a la configuración global (~/.linthis/config.toml)
linthis plugin add -g <alias> <git-url>
linthis plugin add --global <alias> <git-url>
```

### Usar plugin

Los plugins se cargan automáticamente al ejecutar linthis. Después de añadir un plugin:

```bash
# Las configuraciones de plugins se cargan automáticamente
linthis

# Combinar con otras opciones
linthis -i src/
# Solo comprobación
linthis -c
# Solo formateo
linthis -f
# Comprobar y formatear archivos en stage
linthis -s
```

### Eliminar plugin

```bash
# Eliminar plugin de la configuración del proyecto
linthis plugin remove <alias>
linthis plugin remove myplugin

# Eliminar plugin de la configuración global
linthis plugin remove -g <alias>
linthis plugin remove --global myplugin

# Admite orden flexible de parámetros
linthis plugin remove --global myplugin
```

### Ver y gestionar plugins

```bash
# Ver plugins de la configuración del proyecto
linthis plugin list

# Ver plugins de la configuración global
linthis plugin list -g
linthis plugin list --global

# Sincronizar (actualizar) plugins
linthis plugin sync          # Sincronizar plugins locales
linthis plugin sync --global # Sincronizar plugins globales

# Inicializar nuevo plugin
linthis plugin init my-config

# Validar estructura del plugin
linthis plugin validate /path/to/plugin

# Limpiar caché de plugins
linthis plugin clean          # Limpieza interactiva
linthis plugin clean --all    # Limpiar todas las cachés
```

## Archivos de configuración

### Configuración del proyecto

Usa `linthis init` para crear el archivo de configuración:

```bash
linthis init
```

Esto crea `.linthis/config.toml` en la raíz de tu proyecto:

```toml
# Especificar lenguajes a comprobar (omitir para detección automática)
languages = ["rust", "python", "javascript"]

# Excluir archivos y directorios
excludes = [
    "target/**",
    "node_modules/**",
    "*.generated.rs",
    "dist/**"
]

# Máxima complejidad ciclomática
max_complexity = 20

# Preajuste de formato
preset = "google"  # Opciones: google, airbnb, standard

# Configurar plugins
[plugins]
sources = [
    { name = "official" },
    { name = "myplugin", url = "https://github.com/zhlinh/linthis-plugin.git", ref = "main" }
]

# Comprobaciones a ejecutar (por defecto: lint + security + complexity)
[checks]
run = ["lint", "security", "complexity"]

# Configuración de comprobación de seguridad
[checks.security]
scan_type = "sast"    # sca, sast, all
fail_on = "high"      # critical, high, medium, low

# Configuración de comprobación de complejidad
[checks.complexity]
threshold = 15
fail_on_high = true

# Configuración específica por lenguaje
# [rust]
# max_complexity = 15

# [python]
# excludes = ["*_test.py"]
```

### Configuración global

El archivo de configuración global se encuentra en `~/.linthis/config.toml`, con el mismo formato que la configuración del proyecto.

### Prioridad de fusión de configuraciones

Prioridad de fusión (de mayor a menor):

1. **Parámetros CLI**: `--option value`
2. **Configuración del proyecto**: `.linthis/config.toml`
3. **Configuración global**: `~/.linthis/config.toml`
4. **Valores predeterminados integrados**

Para configuraciones específicas de herramientas (ruff.toml, .eslintrc.js, etc.), la prioridad es:

1. **Configuraciones manuales locales** (mayor) - ruff.toml, pyproject.toml, .eslintrc.js en el proyecto
2. **Configuraciones de plugins CLI** - desde la opción `--use-plugin`
3. **Configuraciones de plugins del proyecto** - desde la sección plugins de `.linthis/config.toml`
4. **Configuraciones de plugins globales** - desde `~/.linthis/config.toml`
5. **Valores predeterminados de la herramienta** (menor)

## Gestión de configuración

linthis proporciona un subcomando `config` para la gestión conveniente de la configuración desde la línea de comandos sin editar manualmente manualmente TOML manualmente.

### Operaciones con campos de array

Campos de array soportados: `includes`, `excludes`, `languages`

#### Añadir valores (add)

```bash
# Añadir a la configuración del proyecto
linthis config add includes "src/**"
linthis config add excludes "*.log"
linthis config add languages "rust"

# Añadir a la configuración global (-g o --global)
linthis config add -g includes "lib/**"
linthis config add --global excludes "node_modules/**"

# Añadir múltiples valores (se eliminan duplicados automáticamente)
linthis config add includes "src/**"
linthis config add includes "lib/**"
```

#### Eliminar valores (remove)

```bash
# Eliminar de la configuración del proyecto
linthis config remove excludes "*.log"
linthis config remove languages "python"

# Eliminar de la configuración global
linthis config remove -g includes "lib/**"
linthis config remove --global excludes "target/**"
```

#### Borrar campo (clear)

```bash
# Borrar campo en la configuración del proyecto
linthis config clear languages
linthis config clear includes

# Borrar campo en la configuración global
linthis config clear -g excludes
linthis config clear --global languages
```

### Operaciones con campos escalares

Campos escalares soportados: `max_complexity`, `preset`, `verbose`

#### Establecer valor (set)

```bash
# Establecer límite de complejidad
linthis config set max_complexity 15
linthis config set max_complexity 30 -g

# Establecer preajuste de formato (google, standard, airbnb)
linthis config set preset google
linthis config set preset airbnb --global

# Establecer salida verbose
linthis config set verbose true
linthis config set verbose false -g
```

#### Deshacer valor (unset)

```bash
# Eliminar campo de la configuración del proyecto
linthis config unset max_complexity
linthis config unset preset

# Eliminar campo de la configuración global
linthis config unset -g verbose
linthis config unset --global max_complexity
```

### Operaciones de consulta

#### Obtener valor de un solo campo (get)

```bash
# Ver campo de la configuración del proyecto
linthis config get includes
linthis config get max_complexity
linthis config get preset

# Ver campo de la configuración global
linthis config get -g excludes
linthis config get --global languages
```

#### Listar toda la configuración (list)

```bash
# Listar configuración del proyecto
linthis config list

# Listar configuración global
linthis config list -g
linthis config list --global

# Modo verbose (mostrar todos los campos, incluyendo valores vacíos)
linthis config list -v
linthis config list --verbose
linthis config list --global --verbose
```

### Ejemplos de gestión de configuración

```bash
# Inicializar configuración del proyecto
linthis config add includes "src/**"
linthis config add includes "lib/**"
linthis config add excludes "target/**"
linthis config add excludes "*.log"
linthis config add languages "rust"
linthis config add languages "python"
linthis config set max_complexity 20
linthis config set preset google

# Ver configuración
linthis config list

# Ajustar configuración
linthis config set max_complexity 15
linthis config remove excludes "*.log"
linthis config add excludes "*.tmp"

# Establecer valores globales por defecto
linthis config set -g max_complexity 20
linthis config add -g excludes "node_modules/**"
linthis config add -g excludes ".git/**"
```

### Migración de configuración

linthis puede detectar y migrar automáticamente configuraciones existentes de linter/formateador al formato de linthis.

#### Herramientas soportadas

| Herramienta | Archivos detectados                                                                                                         |
| ----------- | --------------------------------------------------------------------------------------------------------------------------- |
| ESLint      | `.eslintrc.js`, `.eslintrc.json`, `.eslintrc.yml`, `.eslintrc`, `eslint.config.js`, `package.json[eslintConfig]`           |
| Prettier    | `.prettierrc`, `.prettierrc.json`, `.prettierrc.yml`, `.prettierrc.js`, `prettier.config.js`, `package.json[prettier]`     |
| Black       | `pyproject.toml[tool.black]`                                                                                                |
| isort       | `pyproject.toml[tool.isort]`                                                                                                |

#### Comandos de migración

```bash
# Detectar y migrar automáticamente todas las configuraciones
linthis config migrate

# Migrar solo una herramienta específica
linthis config migrate --from eslint
linthis config migrate --from prettier
linthis config migrate --from black
linthis config migrate --from isort

# Vista previa de cambios sin aplicar
linthis config migrate --dry-run

# Crear copia de seguridad de los archivos originales
linthis config migrate --backup

# Salida verbose
linthis config migrate --verbose
```

#### Salida de la migración

Las configuraciones migradas se colocan en `.linthis/configs/{language}/`:

- ESLint → `.linthis/configs/javascript/.eslintrc.js`
- Prettier → `.linthis/configs/javascript/prettierrc.js`
- Black/isort → `.linthis/configs/python/ruff.toml`

### Inicializar archivo de configuración

Usa el subcomando `init` para crear explícitamente archivos de configuración:

```bash
# Crear configuración del proyecto (.linthis/config.toml)
linthis init

# Crear configuración global (~/.linthis/config.toml)
linthis init -g
linthis init --global

# Retrocompatible: también se puede usar la bandera --init
linthis --init
```

### Creación automática de archivos de configuración

Al usar el comando `config`, los archivos de configuración se crean automáticamente si no existen:

- **Configuración del proyecto**: Crea `.linthis/config.toml` en el directorio actual
- **Configuración global**: Crea `config.toml` en el directorio `~/.linthis/`

Todas las modificaciones preservan el formato del archivo TOML y los comentarios.

## Opciones de línea de comandos

### Opciones del comando principal

| Corta | Larga                     | Descripción                                            | Ejemplo                 |
| ----- | ------------------------- | ------------------------------------------------------ | ----------------------- |
| `-i`  | `--include`               | Especificar archivos o directorios a comprobar         | `-i src -i lib`         |
| `-e`  | `--exclude`               | Patrones de exclusión (puede usarse varias veces)      | `-e "*.test.js"`        |
| `-c`  | `--check-only`            | Solo comprobación, sin formateo                        | `-c`                    |
| `-f`  | `--format-only`           | Solo formateo, sin comprobación                        | `-f`                    |
| `-s`  | `--staged`                | Solo comprobar archivos en stage de Git                | `-s`                    |
| `-m`  | `--modified`              | Comprobar todos los archivos modificados localmente    | `-m`                    |
| `-l`  | `--lang`                  | Especificar lenguajes (separados por comas)            | `-l python,rust`        |
| `-o`  | `--output`                | Formato de salida: human, json, github-actions         | `-o json`               |
| `-v`  | `--verbose`               | Salida verbose                                         | `-v`                    |
| `-q`  | `--quiet`                 | Modo silencioso (solo errores)                         | `-q`                    |
|       | `--config`                | Especificar ruta del archivo de configuración          | `--config custom.toml`  |
|       | `--init`                  | Inicializar archivo de configuración .linthis/config.toml | `--init`             |
|       | `--preset`                | Preajuste de formato                                   | `--preset google`       |
|       | `--no-default-excludes`   | Desactivar reglas de exclusión por defecto             | `--no-default-excludes` |
|       | `--no-gitignore`          | Desactivar reglas .gitignore                           | `--no-gitignore`        |
|       | `--no-plugin`             | Omitir carga de plugins, usar configuración por defecto | `--no-plugin`          |

### Subcomandos de gestión de plugins

| Comando                      | Corta | Larga       | Descripción                |
| ---------------------------- | ----- | ----------- | -------------------------- |
| `plugin add <alias> <url>`   | `-g`  | `--global`  | Añadir a configuraciónla configuración global |
|                              |       | `--ref`     | Especificar referencia Git |
| `plugin remove <alias>`      | `-g`  | `--global`  | Eliminar de la configuración global |
| `plugin list`                | `-g`  | `--global`  | Mostrar plugins de la configuración global |
|                              | `-v`  | `--verbose` | Mostrar información detallada |
| `plugin clean`               |       | `--all`     | Limpiar todas las cachés   |
| `plugin init <name>`         |       |             | Inicializar nuevo plugin   |
| `plugin validate <path>`     |       |             | Validar estructura del plugin |

### Subcomandos de gestión de configuración

| Comando                            | Corta | Larga       | Descripción                                  |
| ---------------------------------- | ----- | ----------- | -------------------------------------------- |
| `config add <field> <value>`       | `-g`  | `--global`  | Añadir valor a campo de array                |
| `config remove <field> <value>`    | `-g`  | `--global`  | Eliminar valor de campo de array             |
| `config clear <field>`             | `-g`  | `--global`  | Borrar campo de array                        |
| `config set <field> <value>`       | `-g`  | `--global`  | Establecer valor de campo escalar            |
| `config unset <field>`             | `-g`  | `--global`  | Eliminar campo escalar                       |
| `config get <field>`               | `-g`  | `--global`  | Obtener valor de campo                       |
| `config list`                      | `-g`  | `--global`  | Listar toda la configuración                 |
|                                    | `-v`  | `--verbose` | Mostrar información detallada                |
| `config migrate`                   |       | `--from`    | Migrar desde una herramienta específica    |
|                                    |       | `--dry-run` | Vista previa de cambios sin aplicar          |
|                                    |       | `--backup`  | Crear copia de seguridad de archivos originales |
|                                    | `-v`  | `--verbose` | Mostrar salida detallada                     |

**Campos de array soportados**: `includes`, `excludes`, `languages`
**Campos escalares soportados**: `max_complexity`, `preset`, `verbose`

### Subcomando Init

| Comando | Corta | Larga           | Descripción                      |
| ------- | ----- | --------------- | -------------------------------- |
| `init`  | `-g`  | `--global`      | Crear archivo de configuración global |
|         |       | `--with-hook`   | También instalar gancho git tras la inicialización |
|         |       | `--force`       | Forzar sobrescritura de archivos existentes |

**Archivos de configuración creados**:

- Sin `-g`: Crea `.linthis/config.toml` (directorio actual)
- Con `-g`: Crea `~/.linthis/config.toml` (configuración global)

### Subcomando Hook

<video src="docs/assets/videos/GitHooks-en.mp4" controls width="100%"></video>

| Comando            | Corta | Larga             | Descripción                                                                                                                                                                                                                          |
| ------------------ | ----- | ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `hook install`     |       | `--type`          | Tipo de gancho (git/git-with-agent/agent/prek/prek-with-agent)                                                                                                                                                                            |
|                    |       | `--event`         | Evento de gancho (pre-commit/pre-push/commit-msg)                                                                                                                                                                                          |
|                    | `-g`  | `--global`        | Instalar globalmente: tipo agent → directorio home del usuario; otros → `~/.config/git/hooks/` + `core.hooksPath`                                                                                                                                     |
|                    |       | `--provider`      | Proveedor de IA: `claude`/`codex`/`gemini`/`cursor`/`droid`/`auggie`/`codebuddy`. Admite atajo `provider/model` (ej. `claude/opus`). Para `--type agent`: instala archivos de reglas. Para `*-with-agent`: usa CLI headless para auto-reparación. |
|                    |       | `--provider-args` | Argumentos extra pasados al CLI del agente de IA (ej. `"--model opus"`)                                                                                                                                                                   |
|                    | `-c`  | `--check-only`    | El gancho solo ejecuta comprobación                                                                                                                                                                                                                 |
|                    | `-f`  | `--format-only`   | El gancho solo ejecuta formateo                                                                                                                                                                                                                |
|                    |       | `--force`         | Forzar sobrescritura del gancho existente                                                                                                                                                                                                        |
|                    | `-y`  | `--yes`           | Modo no interactivo                                                                                                                                                                                                                 |
| `hook uninstall`   |       | `--event`         | Evento de gancho a desinstalar                                                                                                                                                                                                              |
|                    | `-g`  | `--global`        | Desinstalar gancho global                                                                                                                                                                                                                |
|                    |       | `--all`           | Desinstalar todos los ganchos                                                                                                                                                                                                                  |
|                    | `-y`  | `--yes`           | Modo no interactivo                                                                                                                                                                                                                 |
| `hook status`      |       |                   | Mostrar estado de los ganchos de git (secciones Project Hooks y Global Hooks)                                                                                                                                                                       |
| `hook check`       |       |                   | Comprobar conflictos de ganchos                                                                                                                                                                                                             |
| `hook sync`        |       |                   | Resincronizar todos los ganchos e habilidades de agentes instalados                                                                                                                                                                                         |
|                    | `-g`  | `--global`        | Sincronizar ganchos globales                                                                                                                                                                                                                    |

**Tipos de ganchos**:

- `git`: Gancho de git tradicional (por defecto)
- `git-with-agent`: Gancho git + auto-reparación con agente de IA en caso de fallo
- `agent`: Gancho de agente de IA (Claude, Cursor, Windsurf, etc.)
- `prek`: Herramienta pre-commit basada en Rust (más rápida)
- `prek-with-agent`: Gancho prek + auto-reparación con agente de IA en caso de fallo

**Ganchos globales**: Usa `-g` / `--global` con cualquier tipo de gancho. Para el tipo `agent`, instala las reglas en el directorio home del usuario. Para todos los demás tipos, instala en `~/.config/git/hooks/` y estableceestablece `git config --global core.hooksPath`. Los ganchos locales tienen prioridad sobre los globales.

<video src="docs/assets/videos/AgentHook-en.mp4" controls width="100%"></video>

**Eventos de gancho**:

- `pre-commit`: Se ejecuta antes del commit (por defecto, comprueba archivos en stage)
- `pre-push`: Se ejecuta antes del push (comprueba todos los archivos)
- `commit-msg`: Valida el formato del mensaje de commit (llama a `linthis cmsg "$1"`)

Cada evento de gancho genera un archivo de habilidad por separado para integraciones de agentes (ej. `lt-lint` para pre-commit, `lt-cmsg` para commit-msg, `lt-review` para pre-push). Todas las habilidades pertenecen a un único paquete de plugin unificado `lt`.

### Subcomando cmsg

Valida el formato del mensaje de commit directamente — sin pasar por un gancho.

| Comando                        | Descripción                                        |
| ------------------------------ | -------------------------------------------------- |
| `cmsg <msg-or-file>`           | Validar una cadena de mensaje de commit o ruta de archivo |
| `cmsg <file> --auto-fix`       | Reescritura con IA en caso de fallo (escribe el resultado de vueltavuelta al archivo) |

```bash
# Validar una cadena de mensaje directamente
linthis cmsg "feat: add new feature"
linthis cmsg "fix(api): handle null response"

# Validar desde un archivo (uso en gancho de git)
linthis cmsg .git/COMMIT_EDITMSG

# Reescritura con IA en caso de fallo (escribe el resultado de vuelta al archivo)
linthis cmsg .git/COMMIT_EDITMSG --auto-fix
linthis cmsg .git/COMMIT_EDITMSG --auto-fix --provider claude-cli

# Instalar el gancho commit-msg (llama a `linthis cmsg "$1"` automáticamente)
linthis hook install --event commit-msg
```

El formato por defecto sigue [Conventional Commits](https://www.conventionalcommits.org/):
`type(scope)?: description` — donde `type` es uno de `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`.

El patrón es configurable vía `.linthis/config.toml`:

```toml
[cmsg]
commit_msg_pattern = "^(feat|fix|docs|...)\\(\\S+\\)?: .{1,72}"
```

### Subcomando format

Formatea archivos con soporte de copia de seguridad y deshacer.

```bash
linthis format                        # Formatear todos los archivos (con copia de seguridad)
linthis format -s                     # Formatear archivos en stage
linthis format -m                     # Formatear archivos modificados
linthis format -i src/main.rs         # Formatear archivo específico
linthis format --undo                 # Deshacer último formateo (restaurar desde copia de seguridad)
linthis format --list-backups         # Listar copias de seguridad disponibles
```

Se crea una copia de seguridad antes de cada operación de formateo. Usa `--undo` para revertir.

> **Nota:** Al ejecutar `linthis -s` (modo staged), los archivos formateados se **reagrupan automáticamente** — no se necesita `git add` manual.

### Subcomando review

Revisión de código impulsada por IA con creación de PR/MR.

```bash
linthis review                        # Revisar rama actual vs remoto
linthis review --auto-fix             # Revisión + auto-reparación + crear PR
linthis review -r alice -r bob        # Especificar revisores
linthis review --base main            | Dif contra la rama main
linthis review --background           | Ejecutar en segundo plano (no bloqueante)
linthis review --status               | Comprobar estado de revisión en segundo plano
linthis review --no-pr                | Generar solo informe en Markdown
```

Plataformas soportadas: **GitHub** (`gh`), **GitLab** (`glab`).

Configurar en `.linthis/config.toml`:

```toml
[review]
enabled = true
provider = "claude-cli"

[review.reviewers]
default = ["alice", "bob"]
```

## Lenguajes soportados

| Language    | Linter                        | Formatter          |
| ----------- | ----------------------------- | ------------------ |
| Rust        | clippy                        | rustfmt            |
| Python      | ruff, pylint, flake8          | ruff, black        |
| TypeScript  | eslint                        | prettier           |
| JavaScript  | eslint                        | prettier           |
| Go          | golangci-lint                 | gofmt              |
| Java        | checkstyle                    | google-java-format |
| C           | clang-tidy, cppcheck          | clang-format       |
| C++         | clang-tidy, cpplint, cppcheck | clang-format       |
| Objective-C | clang-tidy                    | clang-format       |
| Swift       | swiftlint                     | swift-format       |
| Kotlin      | ktlint, detekt                | ktlint             |
| Lua         | luacheck                      | stylua             |
| Dart        | dart analyze                  | dart format        |
| Shell/Bash  | shellcheck                    | shfmt              |
| Ruby        | rubocop                       | rubocop            |
| PHP         | phpcs                         | php-cs-fixer       |
| Scala       | scalafix                      | scalafmt           |
| C#          | dotnet format                 | dotnet format      |

## Plugins de editores

linthis proporciona plugins oficiales para editores populares, ofreciendo una integración perfecta con el formateo al guardar, comandos manuales de lint/format y configuraciones personalizables.

<video src="docs/assets/videos/EditorSkills-en.mp4" controls width="100%"></video>

### VSCode

Instala desde [VS Marketplace](https://marketplace.visualstudio.com/items?itemName=zhlinh.linthis) o busca "linthis" en las extensiones de VSCode.

**Características:**

- Formateo al guardar (configurable)
- Comandos manuales de Lint/Format víavía la paleta de comandos
- Ruta del ejecutable y argumentos adicionales configurables
- Integración con la barra de estado

**Instalación vía paleta de comandos:**

```
ext install zhlinh.linthis
```

**Configuración (settings.json):**

```json
{
  "linthis.formatOnSave": true,
  "linthis.executable.path": "",
  "linthis.executable.additionalArguments": ""
}
```

📁 Código fuente: [vscode-linthis](./vscode-linthis/)

### JetBrains (IntelliJ IDEA, WebStorm, PyCharm, etc.)

Instala desde [JetBrains Marketplace](https://plugins.jetbrains.com/plugin/XXXXX-linthis) o busca "linthis" en Configuración del IDE → Plugins.

**Características:**

- Formateo al guardar (configurable)
- Lint/Format manual a través del menú Herramientas
- Ruta del ejecutable y argumentos adicionales configurables
- Interfaz de usuario en Preferencias → Tools → Linthis

**Instalación:**

1. Abre Configuración/Preferencias → Plugins
2. Busca "linthis"
3. Haz clic en Instalar y reinicia el IDE

**Configuración:**

- Configuración → Tools → Linthis
- Habilitar/deshabilitar Formateo al guardar
- Establecer ruta de ejecutable personalizado
- Añadir argumentos de línea de comandos adicionales

📁 Código fuente: [jetbrains-linthis](./jetbrains-linthis/)

### Neovim

Instala usando tu gestor de plugins favorito. Distribuido vía GitHub.

#### lazy.nvim (Recomendado)

```lua
-- Para monorepositorio (plugin en subdirectorio)
{
  "zhlinh/linthis",
  subdir = "nvim-linthis",
  config = function()
    require("linthis").setup()
  end,
}

-- Para repositorio independiente
{
  "zhlinh/nvim-linthis",
  config = function()
    require("linthis").setup()
  end,
}
```

#### packer.nvim

```lua
-- Para monorepositorio
use {
  "zhlinh/linthis",
  rtp = "nvim-linthis",
  config = function()
    require("linthis").setup()
  end,
}
```

#### vim-plug

```vim
" Para monorepositorio
Plug 'zhlinh/linthis', { 'rtp': 'nvim-linthis' }
```

**Características:**

- Formateo al guardar (configurable)
- Comandos: `:LinthisLint`, `:LinthisFormat`, `:LinthisLintFormat`
- Configurable víavía opciones de `setup()`

**Configuración:**

```lua
require("linthis").setup({
  format_on_save = true,
  executable = "linthis",
  additional_args = {},
})
```

📁 Código fuente: [nvim-linthis](./nvim-linthis/)

## Escenarios de uso

### Gancho pre-commit

#### Método 1: Usar prek (Recomendado para equipos)

[prek](https://prek.j178.dev) es un gestor de ganchos de Git de alto rendimiento escrito en Rust, totalmente compatible con el formato de configuración de pre-commit pero mucho más rápido.

Instalar prek:

```bash
# Usando cargo
cargo install prek

# O usando pip
pip install prek
```

Crear `.pre-commit-config.yaml` en tu proyecto:

```yaml
# .pre-commit-config.yaml
repos:
  - repo: local
    hooks:
      - id: linthis
        name: linthis
        entry: linthis --staged --check-only
        language: system
        pass_filenames: false
```

Instalar gancho:

```bash
prek install
```

#### Método 2: Gancho de Git tradicional (a nivel de proyecto)

Añadir a `.git/hooks/pre-commit`:

```bash
#!/bin/sh
linthis --staged --check-only
```

O usa linthis para crearlo automáticamente:

```bash
linthis hook install --type git
```

#### Método 3: Usar el framework pre-commit

Usando el framework [pre-commit](https://pre-commit.com/):

```yaml
# .pre-commit-config.yaml
repos:
  - repo: local
    hooks:
      - id: linthis
        name: linthis
        entry: linthis --staged --check-only
        language: system
        pass_filenames: false
```

### Integración CI/CD

#### GitHub Actions

```yaml
name: Lint

on: [push, pull_request]

jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Install linthis
        run: pip install linthis
      - name: Run linthis
        run: linthis --check-only --output github-actions
```

#### GitLab CI

```yaml
lint:
  image: rust:latest
  script:
    - cargo install linthis
    - linthis --check-only
```

## Creación de plugins personalizados

### 1. Inicializar plugin

```bash
linthis plugin init my-company-standards
cd my-company-standards
```

### 2. Editar configuración del plugin

Editar `linthis-plugin.toml`:

```toml
[plugin]
name = "my-company-standards"
version = "1.0.0"
description = "Normas de código de mi empresa"

["language.python"]
config_count = 2

["language.python".tools.flake8]
priority = "P0"
files = [".flake8"]

["language.python".tools.black]
priority = "P1"
files = ["pyproject.toml"]
```

### 3. Añadir archivos de configuración

```bash
mkdir -p python
# Añadir tus archivos de configuración a los directorios de lenguaje correspondientes
cp /path/to/.flake8 python/
cp /path/to/pyproject.toml python/
```

### 3b. (Opcional) Agrupar anulaciones de ganchos

Crea `linthis-hook.toml` en la raíz del plugin para enviar ganchos git/de agentes personalizados. Usa `plugin = "self"` — se reemplazará con el alias del usuario cuando añadan el plugin.

```toml
# linthis-hook.toml — anulaciones de ganchos agrupadas
[hook.git]
pre-commit = { source = { plugin = "self", file = "hooks/git/pre-commit" } }

[hook.agent.plugins._default]
"lt" = { source = { plugin = "self", file = "hooks/agent/plugins/lt" } }

[hook.agent.stop]
"claude.settings" = { source = { plugin = "self", file = "hooks/agent/hook/stop/claude/settings.json" } }
```

Coloca los archivos referenciados en el repositorio del plugin. Cuando los usuarios ejecuten `linthis plugin add company <url>`, estas entradas se fusionarán automáticamente en su `.linthis/config.toml`.

### 4. Publicar en Git

```bash
git init
git add .
git commit -m "feat: Initial commit of my company coding standards"
git remote add origin git@github.com:mycompany/linthis-standards.git
git push -u origin main
```

### 5. Usar tu plugin

```bash
linthis plugin add company https://github.com/mycompany/linthis-standards.git
linthis  # Las configuraciones de los plugins se cargan automáticamente
```

## FAQ

### P: ¿Cómo especificar múltiples rutas?

```bash
linthis -i src -i lib -i tests
```

### P: ¿Cómo comprobar solo tipos de archivos específicos?

```bash
linthis -l python  # Solo comprobar archivos Python
```

### P: ¿Dónde está la caché de plugins?

- macOS: `~/Library/Caches/linthis/plugins`
- Linux: `~/.cache/linthis/plugins`
- Windows: `%LOCALAPPDATA%\linthis\cache\plugins`

### P: ¿Cómo actualizar plugins?

```bash
linthis plugin sync          # Sincronizar plugins locales
linthis plugin sync --global # Sincronizar plugins globales
```

### P: ¿Para qué se usa la referencia Git del plugin (ref)?

La ref puede especificar:

- Nombre de rama: `--ref main`
- Etiqueta: `--ref v1.0.0`
- Hash de commit: `--ref abc1234`

Esto permite bloquear versiones de plugins o usar versiones de desarrollo.

## Documentación

- [Sincronización automática de plugins](docs/AUTO_SYNC.md) - Sincronización automática de plugins (inspirado en oh-my-zsh)
- [Autoactualización](docs/SELF_UPDATE.md) - Funcionalidad de autoactualización

## Desarrollo

### Compilar

```bash
cargo build
```

### Probar

```bash
cargo test
```

### Lanzar

```bash
cargo build --release
```

## Contribuir

¡Los issues y Pull Requests son bienvenidos!

## Licencia

Licencia MIT - Ver el archivo [LICENSE](LICENSE) para más detalles
