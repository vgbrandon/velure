# velure

Crea temas de ricing a partir de una imagen, pensados para verse sobrios en un
ambiente profesional, sin importar quién los use.

La imagen aporta el **tono** y el carácter. La **luminosidad** y el **croma** los
decide `velure`, porque son lo que separa un tema elegante de uno chillón. Por eso
una imagen de neón produce un tema tranquilo que sigue pareciéndose a la imagen.

## Instalación

```sh
cargo install --path .
```

## Uso

```sh
velure fondo.jpg                          # informe en el terminal
velure fondo.jpg -x todo                  # exporta todo a ./velure-fondo/
velure fondo.jpg -x kitty,hyprland -o ~/tema
velure fondo.jpg -m oscuro -e sobrio      # fuerza modo e intensidad
velure fondo.jpg -a complementaria        # fuerza una armonía
velure fondo.jpg --json                   # paleta en JSON por stdout
```

| Opción | Valores | Por defecto |
| --- | --- | --- |
| `-m, --modo` | `oscuro`, `claro` | según la luminosidad de la imagen |
| `-e, --estilo` | `sobrio`, `equilibrado`, `expresivo` | `equilibrado` |
| `-a, --armonia` | `mono`, `analoga`, `complementaria`, `dividida`, `ortogonal`, `triadica` | detectada en la imagen |
| `-x, --exportar` | ver la tabla siguiente, o `todo` | ninguno |
| `-o, --salida` | carpeta | `./velure-<imagen>` |

`velure` solo escribe en la carpeta de salida. Nunca toca tu `~/.config`.

## Formatos

| Formato | Archivo | Cómo se usa |
| --- | --- | --- |
| `kitty` | `kitty.conf` | cópialo como `~/.config/kitty/velure-tema.conf` y añade `include velure-tema.conf` a tu `kitty.conf` |
| `hyprland` | `hyprland.conf` | cópialo como `~/.config/hypr/velure-tema.conf`, añade `source = ~/.config/hypr/velure-tema.conf` y usa `$primary`, `$border`... |
| `quickshell` | `Palette.qml` | singleton QML: `Palette.primary`, `Palette.ansi[1]`... |
| `nvim` | `velure.lua` | cópialo a `~/.config/nvim/colors/` y usa `:colorscheme velure` |
| `fish` | `velure.theme` | cópialo a `~/.config/fish/themes/` y usa `fish_config theme choose velure` |
| `starship` | `starship.toml` | copia las líneas a tu `starship.toml` |
| `gtk` | `gtk.css` | `@define-color`, con los nombres de libadwaita |
| `css` | `colors.css` | variables CSS |
| `base16` | `base16.yaml` | esquema para cualquier plantilla base16 |
| `json` | `palette.json` | la paleta completa, con metadatos |
| `preview` | `preview.html` | vista previa con la imagen, la interfaz y la auditoría |

## Teoría aplicada

Todas estas reglas se aplican al generar la paleta. La auditoría comprueba después
el resultado: contraste, croma, carga cromática, separación entre colores y daltonismo.

| Fundamento | Qué hace |
| --- | --- |
| **OKLab / OKLCH** (Ottosson, 2020) | Todo se calcula aquí: pasos iguales de L se ven iguales y el tono no se desplaza al cambiar el croma. |
| **Plantillas armónicas** (Matsuda; ajuste de Cohen-Or et al., 2006) | Prueba 7 plantillas en 360 rotaciones y elige la más estrecha que describe la imagen. |
| **Esquemas clásicos** | Cada plantilla se traduce a monocromática, análoga, complementaria, dividida u ortogonal. La triádica solo se usa si la pides, por ser la más propensa a saturar. |
| **Regla 60-30-10** | Fondo, superficies y acentos: es la base del cálculo de carga cromática. |
| **Equilibrio de áreas** (Moon-Spencer) | Áreas grandes con poco croma y áreas pequeñas con más. Por eso los neutros no se intensifican en el estilo `expresivo`. |
| **Jerarquía cromática** | Un color lidera (100 %) y los demás lo acompañan (80 % y 65 % del croma). |
| **Luminosidad natural del tono** (Munsell) | El amarillo va más claro y el azul más oscuro. El rojo baja para ser terracota y no rosa pastel. |
| **Helmholtz-Kohlrausch** | Los colores saturados se ven más brillantes, así que devuelven algo de L. Es una aproximación de primer orden. |
| **Topes de croma por tono** | Rosas, limas y cianes tienen un techo más bajo porque se ven neón antes que otros tonos. |
| **Contraste WCAG 2 y APCA** | AAA para texto y AA para acentos, ANSI y estados (estos sobre fondo y superficie), medidos sobre los colores finales de 8 bits. |
| **Daltonismo** (Machado et al., 2009) | Los cuatro estados siguen distinguiéndose entre sí bajo protanopia y deuteranopia simuladas. |

Además, los colores de estado se mantienen cerca de su significado (el rojo sigue
siendo rojo) y se apartan de los acentos, para que un botón normal no parezca un error.
Pueden oscurecerse, pero apenas aclararse, porque al aclararse se vuelven pastel.

## Sobre "hombre y mujer"

El color no tiene género, así que `velure` no aplica ninguna regla por género. Lo
que busca es neutralidad: croma contenido, sin rosas ni azules de juguete. Si una
imagen es rosa, el tema será rosa empolvado.

## Como tema del escritorio

`velure aplicar <imagen>` deriva los tokens del escritorio, escribe el tema de cada app y
ejecuta sus ganchos. Por stdout sale `{image, achromatic, tokens}`.

```sh
velure aplicar fondo.jpg [-e estilo] [-a armonia] [--config ARCHIVO] [--sin-ganchos] [--sin-plantillas]
velure tokens fondo.jpg [-o archivo.json]   # los tokens y de qué color de la imagen salen
velure uso                                  # qué tokens usa cada app incorporada
```

### Apps incorporadas

velure escribe por sí mismo el tema de estas apps, con las decisiones de color probadas en
`src/apps/`:

| App | Archivo por defecto |
|---|---|
| `zed` | `~/.config/zed/themes/velure.json` |
| `kitty` | `~/.config/kitty/velure.conf`, que se carga con `globinclude velure.conf` |
| `fish` | `~/.config/fish/velure-colors.fish`, para cargar con `source` desde `config.fish` |
| `starship` | `~/.config/starship/palette-velure.toml`, la paleta `velure` para unir a `starship.toml` |
| `fastfetch` | `~/.config/fastfetch/config.jsonc`, a partir de la base `~/.config/fastfetch/base.jsonc` (cámbiala con `base = '…'`) |
| `eza` | `~/.config/eza/theme.yml`: color solo para lo que significa algo; los permisos quedan neutros |
| `gtk3` | `~/.local/share/themes/Velure/gtk-3.0/colors.css`, los colores que importa el tema GTK3 «Velure» |
| `gtk4` | `~/.config/gtk-4.0/gtk.css`, para libadwaita: `@define-color` y variables CSS (1.6 y posteriores) |
| `iconos` | `~/.local/share/icons/Velure`: Adwaita con las carpetas en el tono del primario (`base` cambia la carpeta de origen) |
| `firefox` | `~/.cache/velure/firefox-*.css`: el marco, las páginas internas y sus versiones translúcidas, para Firefox 158 con el diseño Nova |
| `tidal` | `~/.config/tidal-hifi/themes/velure.css`, basado en el tema Catppuccin de tidal-hifi |

Cada app tiene que cargar el tema: Zed con `"theme": "Velure"` en su configuración y
kitty con `globinclude velure.conf` en `kitty.conf`.

Los CSS de Firefox y Tidal están en `src/apps/firefox/` y `src/apps/tidal.css`, con marcas
`{{token}}`, y van compilados dentro de velure.

### Configuración

`~/.config/velure/config.toml` elige qué escribir. Sin ese archivo, se escriben todas las
apps incorporadas en su ruta por defecto.

```toml
[apps.zed]                        # salida y gancho son opcionales
gancho = 'sh ~/.config/velure/scripts/translucent.sh zed'

[plantillas.mi-app]               # para lo que es propio de tu escritorio
entrada = '~/.config/velure/templates/mi-app.css'
salida = '~/.cache/velure/mi-app.css'
gancho = 'pkill -USR1 mi-app'
```

En una plantilla, `{{primary}}` es el color de ese token e `{{imagen}}` la ruta de la
imagen. Lo que venga pegado detrás del token, como `{{bg}}e0`, queda como canal alfa.
Primero se escriben las apps y después las plantillas, cada grupo en el orden del archivo.
Los archivos se escriben de forma atómica, y una app o plantilla que falla se informa por
stderr sin detener a las demás.

### Tokens

Son 57 y cada color tiene un solo nombre. Se añade un token cuando algo lo necesita. Salen de la paleta auditada, en modo oscuro, y
`CONTRAST_RULES` (`src/tokens.rs`) declara el contraste mínimo de cada par.

| Tokens | Regla |
|---|---|
| `crust`, `bg`, `surface`, `overlay` | Los escalones de la paleta |
| `surface_low`, `surface_high` | Punto medio en OKLab entre sus vecinos |
| `border`, `outline` | Separador de la paleta; contorno a 3:1 como mínimo sobre `surface` |
| `text`, `subtext`, `muted` | Los de la paleta |
| `emphasis`, `on_emphasis` | Casi blanco con el tono del primario, para resaltar sin color |
| `primary`, `secondary`, `tertiary` | Los acentos de la armonía detectada |
| `error`, `warning`, `success`, `info` | Los estados, separados de los acentos |
| `on_primary`, `primary_container`, `on_primary_container` | Texto sobre el primario y su versión de fondo, con texto a 7:1 como mínimo |
| `on_error`, `error_container` | Texto sobre el error y su versión de fondo |
| `on_success`, `on_warning` | Texto sobre un relleno de éxito o de aviso |
| `ansi_0` a `ansi_15` | Los 16 de la paleta, con el significado de cada color |
| `syntax_keyword`, `syntax_function`, `syntax_type` | Los acentos con un croma mínimo, para que un tipo no parezca un comentario |
| `syntax_string`, `syntax_number`, `syntax_property` | Tonos de la imagen lejos de los acentos; si no hay, tonos de reserva |
| (todos los `syntax_*`) | Separados entre sí y del texto por ΔE OKLab de 0,05 como mínimo |
| `gradient_0` a `gradient_7` | Del lado del terciario al del secundario, a 35° como máximo del primario |

## Limitaciones

- Las plantillas de Matsuda se definieron sobre otro círculo cromático. Aquí se
  aplican al de OKLCH, lo que es una adaptación y no el modelo original.
- Los topes de croma, los pasos de luminosidad y el umbral de carga cromática son
  decisiones de diseño, calibradas a ojo sobre imágenes reales. No salen de un estudio.
- La auditoría mide legibilidad y contención, no belleza. Pasar todos los criterios
  no garantiza que el tema te guste: para eso está `preview.html`.
- Si el primario de un tema oscuro es rojizo, `error` tiene poco espacio libre. Para
  seguir viéndose rojo sube de croma, y es el único color que puede superar el tope
  del estilo (hasta 0.16, según el estilo). Aun así puede tirar a naranja rojizo, y en
  `sobrio`, con un tope más bajo, ocasionalmente queda pálido.
- ANSI y Base16 conservan su color semántico, sin apartarse de los acentos.
- El modo claro tiene poco margen de luminosidad para los estados. Por eso usa un ocre
  como amarillo y un verde azulado como verde, y exige menos separación frente a los
  acentos secundarios. En unas pocas imágenes de un solo tono, un estado queda muy oscuro.
- Los acentos de una imagen con un solo tono se separan sobre todo por luminosidad,
  así que se parecen más entre sí.

## Desarrollo

```sh
cargo test
```

`tests/pipeline.rs` pasa imágenes hostiles (neón, arcoíris, gris puro, amarillo
puro...) por todas las combinaciones de modo, estilo y armonía, y exige que se
cumplan todos los criterios de la auditoría.
