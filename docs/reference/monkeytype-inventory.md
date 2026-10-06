# Inventaire des fonctionnalités de Monkeytype (pour un clone TUI en Rust, local)

Source : `github.com/monkeytypegame/monkeytype` (GPL-3.0), `git clone --depth 1`, commit `574d8193498f75d13e7296584c82b64cebe2efea` (6 oct. 2026). Lecture seule, aucun script du dépôt exécuté.
Tous les chemins ci-dessous sont relatifs à la racine du dépôt.

> Remarque d'architecture : le frontend actuel est en SolidJS + TypeScript (`frontend/src/ts/**`). Le moteur de test a été réécrit autour d'un **journal d'événements** (`frontend/src/ts/test/events/*`) : toutes les stats finales sont recalculées à partir de ce journal (keydown/keyup/input/timer/composition horodatés). C'est le modèle le plus simple à transposer en Rust : enregistrer les événements, puis calculer.

---

## 1. Configuration / réglages

- Schéma (zod) : `packages/schemas/src/configs.ts` (`ConfigSchema`, l. ~395-505), groupes `ConfigGroupNameSchema` : `test, behavior, input, sound, caret, appearance, theme, hideElements, hidden, ads`.
- Valeurs par défaut : `frontend/src/ts/constants/default-config.ts`.
- Métadonnées (libellé, description affichée sur la page settings, effets de bord `overrideConfig`, blocages `isBlocked`, `changeRequiresRestart`) : `frontend/src/ts/config/metadata.tsx`.
- Stockage local : config dans localStorage (`frontend/src/ts/config/persistence.ts`), synchro serveur optionnelle (`config/remote.ts`).

Légende : « → » = effets de bord automatiques sur d'autres clés (d'après `overrideConfig` dans `metadata.tsx`).

### 1.1 Test (barre de config du test)

| Clé | Type / valeurs | Défaut | Effet |
|---|---|---|---|
| `mode` | `time \| words \| quote \| zen \| custom` (`packages/schemas/src/shared.ts`) | `time` | Mode de test. → passer à `custom`/`quote`/`zen` force `punctuation=false`, `numbers=false`. Notification si `zen` + pace caret. |
| `time` | entier ≥ 0 (secondes) | `30` | Durée du mode time ; `0` = infini. → force `mode=time`. |
| `words` | entier ≥ 0 | `50` | Nombre de mots du mode words ; `0` = infini. → force `mode=words`. |
| `quoteLength` | tableau de `-3 \| -2 \| 0 \| 1 \| 2 \| 3` | `[1]` | -3 favoris, -2 recherche (quote précise par id), 0 short, 1 medium, 2 long, 3 thicc ; « all » = `[0,1,2,3]` (`config/setters.ts` `setQuoteLengthAll`). → force `mode=quote`. |
| `punctuation` | bool | `false` | Ajoute ponctuation/majuscules aux mots générés. Forcé `false` en mode quote. |
| `numbers` | bool | `false` | 10 % des mots remplacés par un nombre de 1 à 4 chiffres. Forcé `false` en mode quote. |
| `language` | nom de langue (`packages/schemas/src/languages.ts`) | `english` | Liste de mots. Changer la langue relance le test ; arabe → active `lazyMode` si préférence mémorisée. |
| `burstHeatmap` | bool | `false` | Colore l'historique des mots du résultat selon le burst de chaque mot. |

### 1.2 Behavior

| Clé | Type / valeurs | Défaut | Effet |
|---|---|---|---|
| `difficulty` | `normal \| expert \| master` | `normal` | Expert : échec si on valide (espace) un mot incorrect. Master : échec à la première touche incorrecte (100 % requis). |
| `quickRestart` | `off \| esc \| tab \| enter` | `off` | Touche de redémarrage rapide. Avec `esc`, la palette de commandes passe sur `tab`. |
| `repeatQuotes` | `off \| typing` | `off` | `typing` : redémarrer pendant un test quote rejoue la même citation. |
| `resultSaving` | bool | `true` | `false` = s'entraîner sans enregistrer les résultats. |
| `blindMode` | bool | `false` | Aucune erreur surlignée (focus vitesse). |
| `alwaysShowWordsHistory` | bool | `false` | Affiche automatiquement l'historique des mots sur le résultat. |
| `singleListCommandLine` | `manual \| on` | `on` | `on` : palette en liste unique ; `manual` : toutes les commandes seulement après avoir tapé `>`. |
| `minWpm` | `off \| custom` | `off` | Échec si la vitesse passe sous le seuil. |
| `minWpmCustomSpeed` | nombre ≥ 0 (wpm) | `100` | Seuil. → force `minWpm=custom`. |
| `minAcc` | `off \| custom` | `off` | Échec si la précision passe sous le seuil. |
| `minAccCustom` | 0-100 | `90` | Seuil (%). → force `minAcc=custom`. |
| `minBurst` | `off \| fixed \| flex` | `off` | Échec si le burst d'un mot est sous le seuil ; `flex` abaisse le seuil pour les mots longs. |
| `minBurstCustomSpeed` | nombre ≥ 0 | `100` | Seuil burst. |
| `britishEnglish` | bool | `false` | Remplace l'orthographe US par la britannique (liste de remplacements). |
| `funbox` | tableau de `FunboxName` (max 15) | `[]` | Modes « funbox » (voir §4). Bloqué si combinaison invalide (`checkCompatibility`) ou incompatible avec la config (`config/funbox-validation.ts`). |
| `customLayoutfluid` | 2 à 15 layouts | `["qwerty","dvorak","colemak"]` | Layouts parcourus par la funbox layoutfluid (doublons retirés). |
| `customPolyglot` | ≥ 2 langues | `["english","spanish","french","german"]` | Langues de la funbox polyglot. |

### 1.3 Input

| Clé | Type / valeurs | Défaut | Effet |
|---|---|---|---|
| `freedomMode` | bool | `false` | Permet de revenir effacer n'importe quel mot, même correct. → `confidenceMode=off`. |
| `strictSpace` | bool | `false` | Un espace en début de mot insère un caractère espace (au lieu d'être ignoré). |
| `oppositeShiftMode` | `off \| on \| keymap` | `off` | Impose le Shift de la main opposée (erreur sinon) ; ignore B, Y, ^. `keymap` : se base sur `keymapLayout` (pour QMK/émulation). |
| `stopOnError` | `off \| word \| letter` | `off` | `letter` : bloque la saisie sur une lettre fausse ; `word` : impossible de passer au mot suivant tant qu'il y a une erreur. → `confidenceMode=off`, `deleteOnError=off`. |
| `deleteOnError` | `off \| letter \| letter_hard \| word \| word_hard` | `off` | `letter` efface le caractère faux et le précédent ; `word` efface le mot entier ; `_hard` revient en plus au mot précédent si l'erreur est sur le 1er caractère. → `confidenceMode=off`, `stopOnError=off`. |
| `confidenceMode` | `off \| on \| max` | `off` | `on` : impossible de revenir aux mots précédents ; `max` : backspace interdit. → `freedomMode=false`, `stopOnError=off`, `deleteOnError=off`. |
| `quickEnd` | bool | `false` | Mode words : le test finit dès que le dernier mot a la bonne longueur, même faux (sinon il faut un espace). Inactif si stop/delete on error. |
| `indicateTypos` | `off \| below \| replace \| both` | `off` | Montre ce qui a été tapé : sous la lettre, à la place, ou les deux. |
| `compositionDisplay` | `off \| below \| replace` | `replace` | Affichage de la composition IME. |
| `hideExtraLetters` | bool | `false` | Masque les lettres en trop (évite que les mots changent de ligne). |
| `lazyMode` | bool | `false` | Remplace accents/diacritiques par la lettre de base. |
| `layout` | `default` ou nom de layout | `default` | Émulation logicielle d'un autre layout clavier. |
| `codeUnindentOnBackspace` | bool | `false` | Langues `code_*` : backspace sur des tabulations de tête remonte à la ligne précédente. |

### 1.4 Sound

| Clé | Type / valeurs | Défaut | Effet |
|---|---|---|---|
| `soundVolume` | 0-1 | `0.5` | Volume des effets. |
| `playSoundOnClick` | `off` ou `"1"`…`"26"` | `off` | Son par touche : 1 click, 2 beep, 3 pop, 4 nk creams, 5 typewriter, 6 osu, 7 hitmarker, 8 sine, 9 sawtooth, 10 square, 11 triangle, 12 pentatonic, 13 wholetone, 14 fist fight, 15 rubber keys, 16 fart, 17 akko lavenders, 18 cherrymx black abs, 19 cherrymx black pbt, 20 cherrymx blue abs, 21 cherrymx blue pbt, 22 cherrymx brown pbt, 23 kalih box white, 24 razer green, 25 tealios v2, 26 trust gxt. |
| `playSoundOnError` | `off \| 1 \| 2 \| 3 \| 4` | `off` | Son d'erreur : 1 damage, 2 triangle, 3 square, 4 missed punch. Joué sur touche fausse ou espace trop tôt. |
| `playTimeWarning` | `off \| 1 \| 3 \| 5 \| 10` | `off` | Son d'avertissement N secondes avant la fin d'un test chronométré (`test/test-timer.ts` `playTimeWarning`). |

### 1.5 Caret

| Clé | Type / valeurs | Défaut | Effet |
|---|---|---|---|
| `smoothCaret` | `off \| slow \| medium \| fast` | `medium` | Déplacement animé du curseur (durées : voir §7). |
| `caretStyle` | `off \| default \| block \| outline \| underline \| carrot \| banana \| monkey` | `default` | Style du curseur (carrot/banana/monkey cachés de l'UI, `visible:false`). |
| `paceCaret` | `off \| average \| pb \| tagPb \| last \| custom \| daily` | `off` | Second curseur à vitesse constante. average = moyenne des 10 derniers résultats ; tagPb = meilleur PB des tags actifs ; daily = meilleure vitesse des dernières 24 h. pb/tagPb bloqués sans compte. |
| `paceCaretCustomSpeed` | nombre ≥ 0 | `100` | Vitesse du pace caret custom. → `paceCaret=custom`. |
| `paceCaretStyle` | comme `caretStyle` | `default` | Style du pace caret. |
| `repeatedPace` | bool | `true` | En répétant un test, active un pace caret à la vitesse du test précédent (si aucun pace caret actif). |

### 1.6 Appearance

| Clé | Type / valeurs | Défaut | Effet |
|---|---|---|---|
| `timerStyle` (« live progress style ») | `off \| bar \| text \| mini \| flash_text \| flash_mini` | `mini` | Style du timer / compteur de mots ; les `flash_*` n'affichent le timer que brièvement toutes les 15 s en mode time. |
| `liveSpeedStyle` | `off \| text \| mini` | `off` | Vitesse en direct. `text` → `monkey=false`. |
| `liveAccStyle` | `off \| text \| mini` | `off` | Précision en direct. `text` → `monkey=false`. |
| `liveBurstStyle` | `off \| text \| mini` | `off` | Burst du dernier mot en direct. |
| `timerColor` | `black \| sub \| text \| main` | `main` | Couleur du timer et des stats live. |
| `timerOpacity` | `"0.25" \| "0.5" \| "0.75" \| "1"` | `"1"` | Opacité du timer et des stats live. |
| `highlightMode` | `off \| letter \| word \| next_word \| next_two_words \| next_three_words` | `letter` | Ce qui est mis en évidence pendant la frappe. |
| `typedEffect` | `keep \| hide \| fade \| dots` | `keep` | Rendu des mots déjà tapés. |
| `tapeMode` | `off \| letter \| word` | `off` | Une seule ligne défilant horizontalement (à chaque lettre ou mot). → `showAllLines=false`. |
| `tapeMargin` | 10-90 (%) | `50` | Position du curseur depuis la gauche en tape mode. |
| `smoothLineScroll` | bool | `false` | Anime le passage de ligne. |
| `showAllLines` | bool | `false` | Affiche toutes les lignes (words/custom/quote) au lieu de 3. Bloqué avec tapeMode. |
| `alwaysShowDecimalPlaces` | bool | `false` | Décimales toujours visibles sur le résultat. |
| `typingSpeedUnit` | `wpm \| cpm \| wps \| cps \| wph` | `wpm` | Unité d'affichage. Facteurs depuis wpm : cpm ×5, wps ×1/60, cps ×5/60, wph ×60 (`frontend/src/ts/utils/typing-speed-units.ts`). |
| `startGraphsAtZero` | bool | `true` | Axes des graphiques depuis 0. |
| `maxLineWidth` | 0 ou 20-1000 (caractères) | `0` | Largeur max du texte ; 0 = largeur de la zone. |
| `fontSize` | nombre > 0 (rem) | `2` | Taille de police du test. |
| `fontFamily` | nom de police (`packages/schemas/src/fonts.ts`) | `Roboto_Mono` | Police (sans objet en TUI). |
| `keymapMode` | `off \| static \| react \| next` | `off` | Clavier visuel : statique, réagit aux touches pressées, ou montre la prochaine touche. |
| `keymapLayout` | `overrideSync` ou layout | `overrideSync` | Layout affiché (`overrideSync` = suit `layout`). → active `keymapMode=static` si off. |
| `keymapStyle` | `staggered \| alice \| matrix \| split \| split_matrix \| steno \| steno_matrix` | `staggered` | Forme du clavier. |
| `keymapLegendStyle` | `lowercase \| uppercase \| blank \| dynamic` | `lowercase` | Légendes des touches. |
| `keymapKeys` | `minimal \| minimal_numrow \| full` | `minimal` | Rangées affichées. |
| `keymapSize` | 0.5-3.5 (pas 0.1) | `1` | Taille du keymap. |

### 1.7 Theme

| Clé | Type / valeurs | Défaut | Effet |
|---|---|---|---|
| `theme` | nom de thème | `serika_dark` | Thème prédéfini. → `customTheme=false`. |
| `themeLight` / `themeDark` | nom de thème | `serika` / `serika_dark` | Thèmes pour `autoSwitchTheme`. |
| `autoSwitchTheme` | bool | `false` | Bascule clair/sombre selon le système. |
| `randomTheme` | `off \| on \| fav \| light \| dark \| custom \| auto` | `off` | Thème aléatoire après chaque test (non sauvegardé). `custom` exige un compte et des thèmes custom. |
| `favThemes` | tableau de thèmes | `[]` | Favoris. |
| `customTheme` | bool | `false` | Utilise `customThemeColors`. |
| `customThemeColors` | 10 couleurs hex | `#323437, #e2b714, #e2b714, #646669, #2c2e31, #d1d0c5, #ca4754, #7e2a33, #ca4754, #7e2a33` | Ordre : bg, main, caret, sub, subAlt, text, error, errorExtra, colorfulError, colorfulErrorExtra (confirmé : `controllers/theme-controller.ts` l.26-53). Si les 10 sont identiques, retour aux défauts. |
| `flipTestColors` | bool | `false` | Inverse : texte à venir plus clair que le texte tapé. |
| `colorfulMode` | bool | `false` | Les mots utilisent la couleur `main` au lieu de `text`. |
| `customBackground` | URL http(s) d'image (png/gif/jpeg/jpg/webp, ≤ 2048) ou `""` | `""` | Image de fond. |
| `customBackgroundSize` | `cover \| contain \| max` | `cover` | Ajustement de l'image. |
| `customBackgroundFilter` | 4 nombres [blur, brightness, saturate, opacity] | `[0,1,1,1]` | Filtres CSS sur le fond. |

### 1.8 Hide elements

| Clé | Type | Défaut | Effet |
|---|---|---|---|
| `showKeyTips` | bool | `true` | Astuces clavier en pied de page. |
| `showOutOfFocusWarning` | bool | `true` | Avertissement « hors focus » sur le test. |
| `capsLockWarning` | bool | `true` | Avertissement Verr. Maj. |
| `showAverage` | `off \| speed \| acc \| both` | `off` | Affiche la moyenne (10 derniers) à côté de la barre de config. |
| `showPb` | bool | `false` | Affiche le PB pour la config courante. |

### 1.9 Hidden / ads

| Clé | Type / valeurs | Défaut | Effet |
|---|---|---|---|
| `accountChart` | 4 × `on\|off` | `["on","on","on","on"]` | Séries visibles du graphique de l'historique (page compte). |
| `monkey` | bool | `false` | Singe animé qui tape (→ live speed/acc `text` passent à `mini`). |
| `monkeyPowerLevel` | `off \| 1 \| 2 \| 3 \| 4` | `off` | Effets « power » (secousse, particules ; voir §7). |
| `ads` | `off \| result \| on \| sellout` | `result` | Publicités (sans objet). |

### 1.10 Danger zone et autres réglages non-config

La page settings contient aussi des éléments hors `Config` (voir §6.4) : import/export de la config (JSON), réinitialisation des réglages, presets, tags, thèmes custom, polices locales, fond local, cookies.

---

## 2. Modes de test

### 2.1 Les modes

| Mode | Sous-options (UI) | Fin du test | Fichiers |
|---|---|---|---|
| `time` | 15, 30, 60, 120, custom (0 = infini) | Quand `tick >= Config.time` (`checkIfTimeIsUp`, `test/test-timer.ts`) ; `0` : jamais (bail out via palette) | `test/test-timer.ts` |
| `words` | 10, 25, 50, 100, custom (0 = infini) | Dernier mot généré et tapé (voir 2.4) | `input/helpers/fail-or-finish.ts` |
| `quote` | all, short, medium, long, thicc, search (+ favoris) | Fin de la citation | `test/words-generator.ts` `getQuoteWordList` |
| `zen` | aucune | Uniquement manuellement (shift+enter / bail out) ; texte libre, pas de mots cibles | `test/words-generator.ts` `getLimit()` = 0 |
| `custom` | texte + mode + limite (voir 2.3) | Selon la limite word/section/time | `test/custom-text.ts`, `components/modals/CustomTextModal.tsx` |

Durée custom : saisie libre parsée par `parseInput` (`components/modals/CustomTestDurationModal.tsx` l.139) : ex. `1h 30m 10s`, nombres seuls = secondes ; `0` = « Infinite test » (notification rappelant d'utiliser Bail Out), ≥ 1800 s = « Stay safe and take breaks! ».

### 2.2 Génération des mots (`frontend/src/ts/test/words-generator.ts`)

- On génère au départ `getLimit()` mots (l.428) : 100 par défaut, ou moins si `words`/limite custom/citation plus courts ; `showAllLines` génère tout ; propriété funbox `toPush:N` force N.
- Ensuite `addWord()` (`test/test-logic.ts` l.592) maintient ~100 mots d'avance (ou `toPush-1`).
- Tirage aléatoire uniforme dans la liste de la langue (`test/wordset.ts` `randomWord`), ou zipf (funbox `zipf`). Rejet et re-tirage (max 100 fois) si : même mot que l'un des 2 précédents ; `"I"` sans ponctuation ; mot contenant `-=_+[]{};'\:"|,./<>?` sans ponctuation (hors langues code) ; chiffre sans `numbers` (l.~860-885).
- Hors custom/quote, les majuscules sont abaissées sans ponctuation (sauf allemand, suisse allemand, code, klingon).
- Puis : lazy mode, `ß→ss` en suisse allemand, ponctuation, british english, nombres, fonctions funbox ; on ajoute le séparateur `" "` à la fin de chaque mot (sauf funbox `nospace` ou fin par `\n`).

**Numbers** (l.957) : avec probabilité 0,1 le mot est remplacé par `getNumbers(4)` = 1 à 4 chiffres, le premier non nul (`utils/generate.ts` l.180) ; conversion en chiffres arabes-indiens/népalais/bengali/hindi selon la langue.

**Punctuation** (`punctuateWord`, l.46-318), règles dans l'ordre (else-if) :
1. Premier mot ou mot précédent finissant par `. ? ! ؟` → majuscule initiale (sauf code/géorgien) ; espagnol : 10 % `¿`, 10 % `¡` en tête (fermé plus tard).
2. 10 % (si le précédent ne finit pas par `.`/`,` et pas avant-dernier) ou toujours au dernier mot → fin de phrase : 80 % `.`, 10 % `?`, 10 % `!` (variantes `。？！।؟;` selon la langue ; en français `?`, `!`, `:`, `;` deviennent des mots isolés).
3. 1 % guillemets `"mot"` ; 1,1 % `'mot'` ; 1,2 % parenthèses (code : `() {} [] <>` et backticks en JS) ; 1,3 % `:` ; 1,4 % mot remplacé par `-` ; 1,5 % `;` ; 20 % `,` ; code : 25 % symbole (`{ } [ ] ( ) ; = + % /`, plus opérateurs C) ; anglais : 50 % de chance d'appliquer une contraction/apostrophe depuis une liste (`test/english-punctuation.ts`).
(Les probabilités sont évaluées en cascade : chaque `random() < p` n'est atteint que si les précédents ont échoué.)

### 2.3 Custom text (`test/custom-text.ts`, `components/modals/CustomTextModal.tsx`, `packages/schemas/src/util.ts`)

- Réglages stockés en localStorage `customTextSettings` : `{ text: string[], mode: "repeat"|"random"|"shuffle", limit: {mode:"word"|"time"|"section", value}, pipeDelimiter: bool }`. Défaut : « The quick brown fox jumps over the lazy dog », repeat, 9 mots.
- Modes UI : `simple` (= repeat, limite = longueur du texte), `repeat` (dans l'ordre, boucle), `shuffle` (permutation sans remise), `random` (tirage avec remise).
- Limite : nombre de mots, de sections, **ou** durée (secondes). 0 = infini.
- Délimiteur : espace ou `|` (sections : chaque section est un bloc de mots).
- Nettoyages proposés : retirer les caractères de largeur nulle, typographie « fancy » (guillemets courbes, etc.), remplacer les caractères de contrôle (`\n`, `\t` littéraux), remplacer les sauts de ligne par espace ou par « . ». Espaces multiples réduits, sauts de ligne normalisés.
- Textes sauvegardés (`customText` localStorage) et « long texts » avec progression (`customTextLong` : `{text, progress}`) : la progression est sauvegardée en cas d'abandon (`test/test-logic.ts` l.~990-1040).
- Outils : **words filter** (`WordFilterModal.tsx` : langue, longueur min/max, regex, include/exclude, « exact match », presets « home keys, left hand, right hand, home row, top row, bottom row » selon un layout) et **custom generator** (`CustomGeneratorModal.tsx` : jeu de caractères + presets a-z, 0-9, symbols, bigrams, trigrams…, longueur min/max, nombre de mots).

### 2.4 Règles de fin et d'échec

- Fin normale (`checkIfFinished`, `input/helpers/fail-or-finish.ts`) : tous les mots générés **et** on est sur le dernier mot **et** (mot exact **ou** espace/validation **ou** quickEnd avec longueur atteinte et sans stop/delete on error).
- Fin par le temps : chaque tick d'1 s (`timerStep`), `testTime >= maxTime` (mode time ou custom limit time).
- Échecs (`TestLogic.fail(reason)` → `finish(true)`, résultat non sauvegardé, message « Test failed - <raison> ») :
  - `difficulty` : expert (espace sur mot faux, sauf espace sur entrée vide) / master (toute touche fausse). Jamais en zen.
  - `min burst` : à la validation d'un mot, burst < seuil ; `flex` : seuil = `min(speed, floor(speed × 1.03^(-2×(len-3))))` (`whorf`, `utils/misc.ts` l.11).
  - `min speed` : à chaque tick, wpm arrondi < seuil **et** index du mot actif > 3 (`checkIfFailed`, `test/test-timer.ts` l.192).
  - `min accuracy` : à chaque tick, précision live < seuil.
  - `slow timer` : dérive du timer > 500 ms ou > 5 dérives > 250 ms (seulement time < 130 s ou words < 250).
  - `word generation error`.
- Bail out (palette « Bail out ») : termine le test en cours en `bailedOut=true` (pas de PB, AFK non vérifié).

### 2.5 Modes spéciaux

- **Restart** (`restart()`, `test/test-logic.ts` l.197) : nouveau jeu de mots. Bloqué par funbox `no_quit`. Le quick restart est refusé dans les tests « longs » (`utils/quick-restart.ts` : words ≥ 1000 ou 0, time ≥ 900 ou 0, long custom texts) → il faut shift+touche ou la souris. Un test abandonné est comptabilisé comme « incomplete test » (`{acc, seconds}` où seconds = durée − AFK).
- **Repeat test** (`repeatTest()` l.1365) : `restart({withSameWordset:true})` — mêmes mots (replay de `previousGetNextWordReturns`) ; en time/custom-time, si on dépasse les mots du test précédent, la génération aléatoire reprend. Désactivé en zen. Un test répété est marqué « invalid - repeated » (non sauvegardé). `repeatedPace` ajoute un pace caret à la vitesse précédente.
- **Repeat quotes = typing** : redémarrer pendant une citation la rejoue.
- **Practice words** (`test/practise-words.ts`, `PractiseWordsModal.tsx`) : options missed = off/words/biwords, slow = off/on. Limite 20 (missed words seul ou slow seul) sinon 10. Mots ratés triés par nombre d'erreurs ; biwords = (mot précédent + mot raté) ; slow words = mots tapés triés par burst croissant, top `min(limit, round(20 % des mots))`. Chaque mot raté est répété autant de fois qu'il a d'erreurs ; le k-ième mot lent est répété (n−k) fois. Construit un test custom : pipe delimiter, mode shuffle, limit section = (nb d'entrées) × 5, indicateur « practice ». Les réglages précédents (mode, punctuation, numbers, custom text) sont restaurés au restart suivant (« Reverting to previous settings. »).
- **Zen** : pas de cible, chaque saisie est correcte ; durée ajustée en retirant le temps entre la dernière touche et la fin (si < 7 s).
- Anti-triche basique : un retour de focus/visibilité de la fenêtre hors test relance le test (time/words) (l.~1378-1404).

---

## 3. Calculs de statistiques

Fichiers clés :
- `frontend/src/ts/test/events/stats.ts` (toutes les fonctions de calcul depuis le journal d'événements),
- `frontend/src/ts/test/test-logic.ts` `buildCompletedEvent` (l.709-819) et `finish` (l.821-1100),
- `frontend/src/ts/utils/numbers.ts` `calculateWpm` (l.153),
- `packages/util/src/numbers.ts` (`mean`, `stdDev`, `kogasa`),
- `frontend/src/ts/utils/strings.ts` `countChars` (l.417),
- `frontend/src/ts/test/events/live-cache.ts` (précision live).

### 3.1 Formules de base

```
calculateWpm(chars, seconds) = seconds <= 0 ? 0 : chars / 5 / (seconds / 60)
wpm   = round2( calculateWpm(correctWord, duration) )
raw   = round2( calculateWpm(allCorrect + incorrect + extra, duration) )
acc   = round2( correctInputs / (correctInputs + incorrectInputs) * 100 )   // 0 si aucun input
charStats = [correctWord, incorrect, extra, missed]
charTotal = allCorrect + incorrect + extra
```
- La précision compte **chaque frappe d'insertion** (`input` avec champ `correct`) depuis le journal, y compris celles corrigées ensuite (`getAccuracy`, stats.ts l.706). En live, même compteur, 100 % si aucune frappe (`getLiveCachedAccuracy`).
- Les espaces de séparation font partie du mot cible (`"mot "`), donc comptent comme caractères.

### 3.2 Comptage des caractères (`countChars(input, target, creditPartial)`)

Pour chaque position `i < max(len(input), len(target))` :
- `input[i] == target[i]` : si la cible est un espace et le mot n'est pas entièrement correct → `extra`, sinon `allCorrect++` ; et `correctWord++` si le mot est entièrement correct **ou** (creditPartial et la cible commence par l'input).
- `input[i]` absent → `missed++` (sauf creditPartial).
- cible absente, ou cible = espace alors que l'input n'en contient pas → `extra++`.
- sinon → `incorrect++`.

Donc **wpm ne compte que les caractères des mots entièrement corrects** (+ espace) ; raw compte tout ce qui a été tapé (correct, incorrect, extra). Le dernier mot reçoit le crédit partiel en test chronométré, en bail out ou en calcul live (`getChars`, l.631). En coréen, input et cible sont décomposés en jamo (Hangul.disassemble) avant comparaison.

### 3.3 Durée du test (`getTestDurationMs`, l.435)
- = `testMs` de l'événement timer `end` ; en zen ou bail out, on retire (dernière touche → fin) si < 7 s ; arrondi au 1/100 s sauf en custom.
- Contrôle : en mode time (non bail out, ≤ 120 s), si |durée − durée par `Date`| > 0,1 s → « Test invalid - inconsistent test duration ».

### 3.4 Consistency (Kogasa)
```
kogasa(cov) = 100 * (1 - tanh(cov + cov^3/3 + cov^5/5))
consistency    = round2(kogasa( stdDev(rawPerSecond) / mean(rawPerSecond) ))   // 0 si NaN
keyConsistency = round2(kogasa( stdDev(keySpacing[0..n-1]) / mean(...) ))     // dernier intervalle exclu
wpmConsistency = round2(kogasa( stdDev(wpmHistory) / mean(wpmHistory) ))
```
`stdDev` est l'écart-type de **population** (division par n). `rawPerSecond` = burst par seconde (3.6). `keySpacing` = écarts entre keydown successifs (`getKeypressSpacing`).

### 3.5 Burst d'un mot (`computeBurst`, l.511)
- Longueur = longueur de l'input du mot + 1 (le séparateur) si l'input ne finit pas déjà par espace/`\n`.
- Temps = de la 1re insertion au caractère 0 (ou début de composition IME) jusqu'à la dernière insertion du mot.
- `burst = round(calculateWpm(longueur, durée))` ; `Infinity` si durée 0. Calculé à la validation du mot (`input/helpers/word-navigation.ts` l.49) → affichage live burst et contrôle min burst. `getWordBurstHistory` donne la heatmap des mots.

### 3.6 Échantillonnage par seconde (graphique résultat)
- Bornes (`getTimerBoundaries`, l.208) : grille idéale `1000, 2000, … floor(endMs/1000)*1000` ms ; pour les tests non chronométrés, ajout d'une borne finale fractionnaire si le reste ≥ 0,5 s ; zen/bail out : on coupe le temps mort final (< 7 s). Étiquettes `1,2,…`, `7.25` pour la queue, `LAG` si un rattrapage du timer est tombé dans le seau (`getTimerBoundaryLabels`).
- Séries (`chartData`, schéma `packages/schemas/src/results.ts` : max 122 points chacune) :
  - `wpm[i]` = round(calculateWpm(correctWord cumulés jusqu'à la borne i, borne/1000)) — wpm **cumulé** (`getWpmHistory`).
  - `burst[i]` = round(calculateWpm(insertions dans l'intervalle, durée de l'intervalle)) — « raw » par seconde (`getBurstHistory`).
  - `err[i]` = nombre d'insertions incorrectes dans l'intervalle (`getErrorCountHistory`).
  - Il existe aussi `getRawHistory` (raw cumulé) pour d'autres usages.

### 3.7 AFK et validité (`finish`, test-logic.ts l.895-984)
- `afkDuration` = nombre de secondes (seaux) sans aucun keydown ni input (`getAfkDuration`).
- `afkDetected` = les 5 dernières secondes ont 0 insertion (`getKeypressesPerSecond().slice(-5)`), ignoré en bail out.
- Invalidations, dans l'ordre (résultat non sauvegardé ; notification « Test invalid - … ») :
  1. durée incohérente (cf. 3.3) ;
  2. échec difficulté/min… (« Test failed - raison ») ;
  3. **too short** : durée < 1 s ; time < 15 s (ou time infini et durée < 15) ; words < 10 (ou infini et durée < 15) ; custom limite word/section < 10 ou time < 15 ; zen < 15 s ;
  4. **AFK detected** ;
  5. **repeated** ;
  6. **wpm** < 0 ou > 350 (> 420 pour words 10) ; idem **raw** ;
  7. **accuracy** < 75 % (ou < 50 % si opt-out leaderboard) ou > 100.
- Message sarcastique + confettis si wpm = 0 et durée ≥ 5 s.
- Temps tapé du jour (`test/today-tracker.ts`) += durée − AFK.

### 3.8 Live stats (pendant le test)
À chaque tick d'1 s (`timerStep`, test-timer.ts l.270) : `wpm = round(calculateWpm(correctWord avec crédit partiel, ms écoulées/1000))`, `raw` idem avec allCorrect+extra+incorrect, `acc` = précision live. Burst live mis à jour à chaque validation de mot.

### 3.9 Personal bests
- Éligibilité locale (`test/result-pb.ts` `getPbEligibility`) : pas en quote, résultat sauvegardable, funboxes toutes `canGetPb`, pas de stopOnError=letter sous 100 %, pas de bail out.
- Clé de PB (`backend/src/utils/pb.ts` `matchesPersonalBest`) : (mode, mode2, punctuation, numbers, language, difficulty, lazyMode). Mise à jour si wpm strictement supérieur ; on stocke `{acc, consistency, difficulty, lazyMode, language, punctuation, numbers, raw, wpm, timestamp}` (`packages/schemas/src/shared.ts` `PersonalBestSchema`).
- `mode2` = durée (time), nombre de mots (words), id de citation (quote), `custom`, `zen` (`utils/misc.ts` `getMode2`).

### 3.10 Autres mesures enregistrées
`keySpacing`, `keyDuration` (keydown→keyup par touche), `keyOverlap` (temps total avec ≥ 2 touches enfoncées), `startToFirstKey`, `lastKeyToEnd`, `restartCount`, `incompleteTests`, `incompleteTestSeconds`, `afkDuration` (stats.ts l.742-1028 ; test-logic.ts l.776-819).

### 2.6 Règles de saisie utiles au moteur (`frontend/src/ts/input/handlers/*`)
- Espace sur un mot vide : ignoré, sauf `strictSpace`, difficulté ≠ normal, ou `deleteOnError` `*_hard` (`before-insert-text.ts` l.~66-73).
- `\n` accepté seulement si le texte contient des retours à la ligne, ou en zen.
- Limite de saisie par mot : longueur cible + 20 caractères (30 en zen) ; une lettre en trop qui ferait passer le mot à la ligne suivante est refusée (sauf blind/hideExtraLetters).
- Backspace (`before-delete.ts`) : impossible de revenir au mot précédent s'il est correct (sauf `freedomMode`) ; `confidenceMode=on` interdit de revenir au mot précédent, `max` interdit tout backspace ; ctrl+backspace = `deleteWordBackward`.
- Validation du mot (espace) : avance même si le mot est faux (sauf `stopOnError=word`) ; le mot faux est souligné et reste corrigeable.
- Mode code : tabulations auto-insérées en début de ligne (`insert-text.ts` l.~325-335).

---

## 4. Funboxes

Fichiers :
- `packages/funbox/src/list.ts` (métadonnées des 48 funboxes),
- `packages/funbox/src/types.ts` (propriétés),
- `packages/funbox/src/validation.ts` (`checkForcedConfig` l.7-70, `checkCompatibility` l.72-245),
- `frontend/src/ts/test/funbox/funbox-functions.ts` (implémentations, noté FF ci-dessous),
- `frontend/src/ts/test/funbox/funbox.ts` (activation, CSS),
- `frontend/src/ts/config/funbox-validation.ts`,
- CSS : `frontend/static/funbox/*.css` (15 fichiers).

Champs de métadonnées : `name`, `description`, `canGetPb` (bool), `difficultyLevel` (0-3, sert uniquement au bonus d'XP côté serveur), `properties`, `frontendForcedConfig`, `frontendFunctions`, `cssModifications` (`main | typingTest | words | body`). `Config.funbox` accepte au plus 15 funboxes.

### 4.1 Propriétés
- `hasCssFile` : une feuille CSS `funbox/<nom>.css` est chargée.
- `ignoreReducedMotion` : l'animation reste active même avec « reduced motion ».
- `noJoiningScript` : désactivée pour les écritures liées (arabe, etc.).
- `noInfiniteDuration` : `words=0` passe à 10, `time=0` passe à 15.
- `toPush:N` : seulement N−1 mots générés après le mot courant (donc visibles).
- `wordOrder:reverse` : ordre des mots inversé.
- `reverseDirection` : sens LTR/RTL inversé.
- `nospace` : la touche espace est bloquée ; la dernière lettre valide le mot.
- Propriétés utilisées seulement par les règles de compatibilité : `changesLayout`, `usesLayout`, `ignoresLayout`, `ignoresLanguage`, `changesWordsVisibility`, `changesWordsFrequency`, `changesCapitalisation`, `noLetters`, `symmetricChars`, `conflictsWithSymmetricChars`, `speaks`, `unspeakable`.

Fonctions (hooks) : `getWord`, `punctuateWord`, `withWords`, `pullSection`, `alterText` (s'enchaînent), `getWordsFrequencyMode`, `getWordHtml`, `getEmulatedChar`, `handleKeydown`, `handleSpace`, `toggleScript` (TTS), `start`, `restart`, `applyConfig`, `rememberSettings` (sauvegarde puis restaure la config d'origine), `applyGlobalCSS` / `clearGlobal`, `getResultContent`.

### 4.2 Liste complète (48)

Colonnes : PB = `canGetPb`, D = `difficultyLevel`.

| Nom | PB | D | Propriétés / config forcée | Effet |
|---|---|---|---|---|
| `58008` | non | 1 | ignoresLanguage, ignoresLayout, noLetters ; numbers=false | Les mots sont des nombres de 1 à 7 chiffres. Avec ponctuation : 50 % de chance d'un `.` interne (mots > 3 car.) et 75 % de chance d'un opérateur `/ * - +`. Entrée = espace. |
| `mirror` | oui | 3 | hasCssFile (main) | Page en miroir horizontal. |
| `upside_down` | oui | 3 | hasCssFile (main) | Page à l'envers. |
| `nausea` | oui | 2 | hasCssFile, ignoreReducedMotion (typingTest) | Oscillation 3D du test (7 s), en-tête et pied inclinés. |
| `round_round_baby` | oui | 3 | hasCssFile, ignoreReducedMotion (typingTest) | Le test tourne de 360° toutes les 5 s. |
| `simon_says` | oui | 1 | hasCssFile, changesWordsVisibility, usesLayout ; highlightMode ∈ {letter, off} | Lettres à venir invisibles ; keymap en mode `next` : on tape en suivant le clavier. |
| `tts` | oui | 1 | hasCssFile, changesWordsVisibility, speaks (words) | Lettres invisibles ; chaque mot est lu par synthèse vocale (bcp47 de la langue). |
| `choo_choo` | oui | 2 | hasCssFile, noJoiningScript, conflictsWithSymmetricChars, ignoreReducedMotion (words) | Chaque lettre tourne sur elle-même (2 s). |
| `arrows` | non | 1 | ignoresLanguage, ignoresLayout, nospace, noLetters, symmetricChars ; punctuation/numbers=false | Séquences de flèches façon DDR. Touches : a/←/j, s/↓/k, w/↑/i, d/→/l. |
| `rAnDoMcAsE` | non | 2 | changesCapitalisation | Chaque lettre a 50 % de chance d'être en majuscule. |
| `sPoNgEcAsE` | non | 2 | changesCapitalisation | Alternance minuscule/majuscule. |
| `capitals` | non | 1 | changesCapitalisation | Majuscule initiale à chaque mot. |
| `layout_mirror` | oui | 3 | changesLayout | Layout en miroir (sur chaque rangée, les 10-11 premières touches sont inversées). |
| `layoutfluid` | oui | 1 | changesLayout, noInfiniteDuration | Change de layout (liste `customLayoutfluid`) à intervalles égaux du test. Compte à rebours 3 secondes (mode time) ou 3 mots avant chaque changement. |
| `earthquake` | oui | 1 | hasCssFile, noJoiningScript, ignoreReducedMotion (words) | Les lettres tremblent. |
| `space_balls` | oui | 0 | hasCssFile, ignoreReducedMotion (body) | Fond étoilé et texte incliné façon Star Wars. |
| `gibberish` | non | 1 | ignoresLanguage, unspeakable | Chaînes a-z aléatoires de 1 à 7 lettres. |
| `ascii` | non | 1 | ignoresLanguage, noLetters, unspeakable ; punctuation/numbers=false | ASCII imprimable (33-126), 1 à 10 caractères. |
| `specials` | non | 1 | idem ascii | 1 à 7 symboles. |
| `plus_zero` / `plus_one` / `plus_two` / `plus_three` | oui | 1/0/0/0 | changesWordsVisibility, toPush:1/2/3/4, noInfiniteDuration | Seuls le mot courant et les 0/1/2/3 mots suivants existent. |
| `read_ahead_easy` / `read_ahead` / `read_ahead_hard` | oui | 1/2/3 | changesWordsVisibility, hasCssFile ; highlightMode ∈ {letter, off} | Le mot courant (puis aussi +1, +2 mots) est invisible ; visible après backspace sur une erreur, de nouveau masqué à l'espace. |
| `memory` | oui | 3 | changesWordsVisibility, noInfiniteDuration ; mode ∈ {words, quote, custom} | Mots montrés pendant `round(nbMots^1.2)` s, puis masqués dès que la frappe commence ou que le temps est écoulé ; force showAllLines. |
| `nospace` | non | 0 | nospace ; highlightMode ∈ {letter, off} | Mots collés, sans espace. |
| `poetry` | non | 0 | noInfiniteDuration, ignoresLanguage ; punct/numbers=false | Poème récupéré sur poetrydb.org (réseau). |
| `wikipedia` | non | 0 | idem | Section Wikipédia aléatoire (réseau). |
| `weakspot` | non | 0 | changesWordsFrequency | Score par caractère = moyenne mobile (fenêtre 50) du temps de frappe, +5000 ms par erreur ; chaque mot est le meilleur de 20 tirages (`test/weak-spot.ts`). |
| `pseudolang` | non | 0 | unspeakable, ignoresLanguage | Mots inventés par chaîne de Markov sur les caractères de la langue (préfixe de 2). |
| `IPv4` / `IPv6` | non | 1 | ignoresLanguage, ignoresLayout, noLetters ; numbers=false | Adresses IP aléatoires (25 % de notation CIDR avec ponctuation). |
| `binary` | non | 1 | idem ; punctuation=false | Octets en binaire sur 8 bits. |
| `hexadecimal` | non | 1 | idem | 1 à 4 octets en hexadécimal (`0x` avec ponctuation). |
| `zipf` | non | 0 | changesWordsFrequency | Tirage selon la loi de Zipf (`utils/misc.ts` l.419-433). Prévient si la liste n'est pas triée par fréquence. |
| `morse` | non | 1 | ignoresLanguage, ignoresLayout, noLetters, nospace | Chaque caractère est converti en morse suivi de `/`. |
| `crt` | oui | 0 | hasCssFile, noJoiningScript (body) | Lignes de balayage et lueur façon écran CRT. |
| `backwards` | oui | 3 | hasCssFile, conflictsWithSymmetricChars, wordOrder:reverse, reverseDirection (words) | Lettres et ordre des mots inversés, sens de lecture inversé. |
| `ddoouubblleedd` | oui | 1 | noJoiningScript | Chaque caractère est doublé. |
| `instant_messaging` | non | 0 | changesCapitalisation | Tout en minuscules ; `.!?` final remplacé par un retour à la ligne ; `( ) . ' "` supprimés. |
| `underscore_spaces` | non | 1 | ignoresLanguage, ignoresLayout, nospace | `_` à la fin de chaque mot. |
| `ALL_CAPS` | non | 1 | changesCapitalisation | Tout en majuscules. |
| `polyglot` | non | 1 | ignoresLanguage | Mélange les mots des langues de `customPolyglot`. |
| `asl` | oui | 1 | hasCssFile, noJoiningScript (words) | Police Gallaudet (alphabet dactylologique de la langue des signes américaine). |
| `rot13` | oui | 1 | — | Texte chiffré en ROT13. |
| `no_quit` | oui | 0 | — | Interdit restart, navigation et changement de config pendant un test. |

`canGetPb = true` pour 25 funboxes, `false` pour 23 (comptage par grep dans `list.ts`).

### 4.3 Compatibilité (`checkCompatibility`)
Toutes ces règles doivent être vraies :
- au plus 1 funbox parmi `getWord` / `pullSection` / `withWords` ;
- au plus 1 `wordOrder` ;
- pas `changesLayout` avec `ignoresLayout` ou `usesLayout` ;
- au plus 1 funbox `nospace` ou `toPush` ;
- au plus 1 `changesWordsVisibility` ;
- au plus 1 `changesWordsFrequency`, et jamais avec `ignoresLanguage` ;
- pas `noLetters` avec `changesCapitalisation` ;
- pas `conflictsWithSymmetricChars` avec `symmetricChars` ;
- au plus 1 `speaks`, et pas de `speaks` avec `unspeakable` (lorsque `ignoresLanguage` est présent) ;
- au plus 1 `toPush` ou `pullSection` ;
- au plus 1 funbox pour chacun de : `punctuateWord`, `getEmulatedChar`, `getWordHtml`, `changesCapitalisation` ;
- au plus 1 funbox par cible `cssModifications` ;
- intersection non vide des `frontendForcedConfig`.

Compatibilité avec la config (`config/funbox-validation.ts`) :
- zen est interdit avec les funboxes qui génèrent ou modifient les mots, `nospace`, `toPush`, la visibilité, `speaks`, `changesLayout` ou la fréquence ;
- quote et custom sont interdits avec `getWord` / `pullSection` / `withWords` / `changesWordsFrequency`.

### 4.4 Challenges
- Définis en TypeScript : `packages/challenges/src/index.ts` (58 challenges ; noms dans `packages/schemas/src/challenges.ts`).
- Textes de scripts : `frontend/static/challenges/*.txt` (13 fichiers, 1,2 Mo).
- Logique : `frontend/src/ts/controllers/challenge-controller.ts` (`setup` l.204, `verify` l.145).
- Format : `{name, display, description, category, discordRoleId, type, parameters, requirements?, settings?}`.
  - `category` : endurance 7, script 19, speed 4, accuracy 3, funbox 16, other 9.
  - `type` : customTime, customWords, customText, script, accuracy, funbox ou other.
  - `requirements` : `wpm{min|exact}`, `acc{min|exact}`, `raw{exact}`, `con{exact}`, `afk{max}`, `time{min}`, `funbox{exact}`, `config` partiel.
- Exemples : « One Hour Warrior » (time 3600) ; « Accuracy Expert » (durée infinie, difficulté master, ≥ 60 wpm, 100 %, AFK ≤ 5 %, ≥ 600 s) ; « 69 » (wpm = raw = acc = con = 69).
- Vérification locale : échec si l'AFK dépasse 10 % de la durée, puis contrôle de chaque exigence. Côté serveur, seule l'attribution d'un rôle Discord est faite. Tout est faisable en local.

---

## 5. Données

Taille totale de `frontend/static` : environ 155 Mo pour 1 092 fichiers.

| Dossier | Taille | Fichiers |
|---|---|---|
| `languages/` | 135 Mo | 446 |
| `quotes/` | 7,2 Mo | 87 |
| `sounds/` | 5,4 Mo | 155 |
| `webfonts/` | 4,3 Mo | 42 |
| `challenges/` | 1,2 Mo | 13 |
| `layouts/` | 956 Ko | 239 |
| `themes/` (CSS) | 216 Ko | 52 |
| `funbox/` (CSS) | 60 Ko | 15 |

Langues et citations pèsent ensemble environ 142 Mo. Sans les listes géantes (`*_450k`, `*_600k`…), on tombe à quelques Mo.

### 5.1 Langues : `frontend/static/languages/<nom>.json` (446 fichiers)
Schéma `LanguageObjectSchema` (`packages/schemas/src/languages.ts`, strict) :
```json
{ "name": "english", "noLazyMode": true, "orderedByFrequency": true,
  "words": ["the","be","of","and","a","to", "..."] }
```
- Champs optionnels : `rightToLeft`, `noLazyMode`, `joiningScript`, `orderedByFrequency`, `additionalAccents: [[accent, remplacement], ...]` (ex. allemand `[["ä","ae"],["ö","oe"],["ü","ue"]]`), `bcp47` (ex. `fr-FR`), `preferredFont`, `originalPunctuation`.
- Il n'y a pas de champ `ligatures`.
- Tailles de la famille anglais :
  - `english` : 200 mots ;
  - `english_1k` : 1 000 mots ;
  - `english_5k`, `english_10k`, `english_25k` ;
  - `english_450k` : 7,9 Mo, ordre alphabétique.
- Groupes (`english`, `french` : `_1k`, `_2k`, `_10k`, `_600k`, `_bitoduc`… ; 69 langues `code_*` ; environ 115 groupes) : `frontend/src/ts/constants/languages.ts`.
- `removeLanguageSize` (`utils/strings.ts` l.165) retire `_\d*k` pour trouver le fichier de citations.
- Lazy mode (`frontend/src/ts/test/lazy-mode.ts`) : table d'accents codée en dur (`áàâä…→a`, `ß→ss`, `æ→ae`, `œ→oe`, diacritiques arabes supprimés…), complétée par les `additionalAccents` de la langue, qui sont prioritaires. La casse est conservée. Le lazy mode est inactif si la langue a `noLazyMode`, sauf en mode custom.

### 5.2 Citations : `frontend/static/quotes/<langue>.json` (87 fichiers, 15 828 citations)
Schéma `QuoteDataSchema` (`packages/schemas/src/quotes.ts`) :
```json
{ "language": "english",
  "groups": [[0,100],[101,300],[301,600],[601,9999]],
  "quotes": [ { "text": "You have the power to heal your life, and you need to know that.",
                "source": "Meditations to Heal Your Life", "length": 64, "id": 1 } ] }
```
- Champs optionnels d'une citation : `britishText`, `approvedBy`.
- Groupes : index 0..3 = short, medium, long, thicc. Une citation appartient au groupe i si `lower <= length <= upper` (`controllers/quotes-controller.ts` l.85-97).
- Cinq fichiers ont leurs propres bornes : chinese_simplified, korean, code_vhdl, code_arduino, code_systemverilog.
- `english.json` : 6 488 citations, 2,3 Mo (short 926, medium 3 786, long 1 589, thicc 187).
- Normalisation au chargement (`words-generator.ts` l.~565) : espaces multiples réduits, `\n` normalisés en `"\n "`, `…` remplacé par `...`.

### 5.3 Thèmes
- Noms : `packages/schemas/src/themes.ts` (187 thèmes).
- Couleurs : `frontend/src/ts/constants/themes.ts`, un objet par thème :
  `{ bg, main, caret, sub, subAlt, text, error, errorExtra, colorfulError, colorfulErrorExtra, hasCss? }`
- Ordre de `customThemeColors` : `[bg, main, caret, sub, subAlt, text, error, errorExtra, colorfulError, colorfulErrorExtra]` (`controllers/theme-controller.ts` l.26-53).
- Exemple `serika_dark` : bg `#323437`, main `#e2b714`, caret `#e2b714`, sub `#646669`, subAlt `#2c2e31`, text `#d1d0c5`, error `#ca4754`, errorExtra `#7e2a33`, colorfulError `#ca4754`, colorfulErrorExtra `#7e2a33`.
- Variables CSS : `--bg-color`, `--main-color`, `--caret-color`, `--sub-color`, `--sub-alt-color`, `--text-color`, `--error-color`, `--error-extra-color`, `--colorful-error-color`, `--colorful-error-extra-color`.
- 52 thèmes ont un CSS décoratif supplémentaire (`frontend/static/themes/*.css`), non transposable en TUI.

### 5.4 Layouts clavier : `frontend/static/layouts/<nom>.json` (239 fichiers ; 196 ansi, 43 iso)
```json
{ "keymapShowTopRow": false, "type": "ansi",
  "keys": { "row1": [["`","~"],["1","!"],"..."], "row2": [["q","Q"],"..."],
            "row3": [["a","A"],"..."], "row4": [["z","Z"],"..."], "row5": [[" "]] } }
```
- Chaque touche a 1 à 4 légendes : [normal, shift, altgr, altgr+shift].
- Nombre de touches par rangée : ansi 13/13/11/10/1-2, iso 13/12/12/11/1-2.
- Il n'y a pas de type « matrix » : les layouts `*_matrix` sont de type ansi.
- Schéma : `packages/schemas/src/layouts.ts`.
- Émulation de layout : `frontend/src/ts/test/layout-emulator.ts`.

### 5.5 Sons : `frontend/static/sounds/` (155 fichiers `.wav`, 5,4 Mo)
- Clics : `click{N}/{1..k}.wav`.
  - click1, 2, 3, 6, 7 : 3 fichiers chacun ;
  - click4, click5 : 6 fichiers chacun ;
  - click14 : 8, click15 : 5, click16 : 8 ;
  - click17 à click26 : 10 fichiers chacun.
- Correspondance : `frontend/src/ts/constants/sounds.ts`. Un fichier est joué au hasard à chaque frappe.
- Erreurs : `error1/1.wav`, `error2/1.wav`, `error3/1.wav`, `error4/{1,2}.wav`.
- Autres fichiers : `timeWarning.wav`, `fart-reverb.wav`.
- Sons synthétisés (pas de fichiers ; `controllers/sound-controller.ts`) :
  - 8 à 11 (sine, sawtooth, square, triangle) : la touche (`event.code`) est associée à une note, disposée comme un piano (rangée ZXCV… = do à si, octave de base 3, +1 avec Shift). Gain = `volume/10`, décroissance `setTargetAtTime(0, t, 0.15)`, arrêt à t + 0,5 s.
  - 12 (pentatonique do-ré-mi-sol-la) et 13 (gamme par tons) : note aléatoire dans la gamme, sinus, octave qui oscille entre 4 et 6, décroissance 0,3, arrêt à t + 2 s.
- Lecture via Howler ; volume = `soundVolume`.

### 5.6 Divers
- Polices : 43 polices connues (`packages/schemas/src/fonts.ts`), sans objet en TUI.
- British English : `frontend/src/ts/constants/british-english.ts`, environ 671 entrées `{american: british | {britishWord, exceptPreviousWords}}`. Logique : `test/british-english.ts`.
- Funboxes `poetry` et `wikipedia` : API réseau (poetrydb.org, wikipedia REST). À exclure dans un clone 100 % local, ou à remplacer.
- Il n'y a pas de listes de mots propres aux funboxes : tout est généré dans le code.

---

## 6. UI / écrans

Chemins relatifs à `frontend/src/ts/`.

### 6.1 Barre de config du test (`components/pages/test/TestConfig.tsx`)
La barre a trois groupes : `[punctuation | numbers]` · `[time | words | quote | zen | custom]` · options du mode courant.
- **time** : 15, 30, 60, 120, bouton outils (durée custom, modal `TestDuration`).
- **words** : 10, 25, 50, 100, bouton outils (nombre custom).
- **quote** : all, short, medium, long, thicc, favoris (cœur, compte requis), recherche (modal QuoteSearch).
- **custom** : bouton « change » (modal CustomText).
- **zen** : le groupe punctuation/numbers est masqué et il n'y a pas d'options.
- En quote, punctuation et numbers sont désactivés.
- La barre passe à opacité 0 en focus mode et sur l'écran de résultat.

**Badges au-dessus du test** (`components/pages/test/modes-notice/`), dans l'ordre :
1. repeated
2. saving disabled
3. rappels de touches (tab/esc/enter)
4. texte long (« Shift+Enter to save progress »)
5. challenge chargé
6. « Shift+Enter to finish zen »
7. langue (ou langues polyglot)
8. expert/master
9. blind
10. lazy
11. pace (« {type} pace {vitesse} {unité} »)
12. moyenne des 10 derniers (`showAverage`)
13. PB de la config (`showPb`)
14. min speed / min acc / min burst
15. funboxes
16. confidence
17. stop on error / delete on error
18. « emulating {layout} »
19. opposite shift
20. tags

Les badges cliquables ouvrent la palette sur le sous-menu correspondant.

Autres éléments de la page test : avertissement Caps Lock, avertissement hors focus, affichage de composition IME, keymap, monkey, stats live.

### 6.2 Palette de commandes (`commandline/lists.ts`, `commandline/lists/*.ts`, `commandline/filter.ts`, `components/modals/CommandlineModal.tsx`)

**Ouverture** (`input/hotkeys/commandline.ts`, `states/hotkeys.ts`) :
- `Esc`, ou `Tab` si `quickRestart=esc` ; `Shift+Tab` si les mots contiennent des tabulations.
- `Ctrl/Cmd+Shift+P` dans tous les cas.

**Catégories racine, dans l'ordre :**
1. **Écran de résultat** (visible seulement sur le résultat) : Next test, Repeat test, Practice words… (missed / slow / both / custom), Toggle word history, Copy screenshot, Download screenshot, Copy words to clipboard.
2. **Test** :
   - sous-menus punctuation, numbers, mode, time, words, quoteLength, language ;
   - Change custom text, Search for quotes ;
   - ajouter/retirer la citation courante des favoris ;
   - **Bail out…** (Nevermind / Yes, I am sure ; seulement pour les tests longs, infinis ou zen) ;
   - Share test settings.
3. **Compte** : Tags… (clear, bascule par tag, create), Presets… (appliquer, create).
4. **Behavior** : une entrée par clé du §1.2, plus Funbox… (none + chaque funbox) et Minimum word burst…
5. **Input** : clés du §1.3.
6. **Sound** : clés du §1.4.
7. **Caret** : clés du §1.5.
8. **Appearance** : clés du §1.6, plus Custom font… (nom, police locale, retirer).
9. **Theme** :
   - Theme… (favoris en tête, aperçu au survol), customTheme, Custom themes… ;
   - flipTestColors, colorfulMode ;
   - ajouter/retirer le thème des favoris ;
   - Custom background… (fichier local, retirer), customBackgroundSize ;
   - Custom background filter… (blur, brightness, saturation, opacity) ;
   - randomTheme, Next random theme.
10. **Show/hide** : showKeyTips, showOutOfFocusWarning, capsLockWarning, showAverage, showPb, monkeyPowerLevel, monkey.
11. **Danger zone** : ads.
12. **Other** :
    - Load challenge… ;
    - navigation (Typing, Leaderboards, About, Settings, Account, Search profile, **Toggle Fullscreen**) ;
    - Import settings JSON / Export settings JSON ;
    - Clear all notifications ;
    - entrées cachées : Copy last event log, FPS counter ;
    - Discord, Sign out.

**Libellé des sous-menus de config** : `Capitalize(displayString ?? clé) + "..."`. Les options viennent du schéma zod ; `off`/`false` est placé en premier (`commandline/util.ts`).

**Navigation dans la palette** :
- ↑ / Ctrl+K / Ctrl+P : élément précédent ; ↓ / Ctrl+J / Ctrl+N : élément suivant ; Tab / Shift+Tab : idem ;
- Entrée : exécuter ;
- Échap : remonter d'un sous-menu, ou fermer.
- Il existe aussi un mode « input » pour les commandes qui demandent du texte libre (avec validation).

**singleListCommandLine** :
- `on` : tous les sous-menus sont aplatis en « Parent Enfant ».
- `manual` : l'aplatissement n'a lieu que si la saisie commence par `>`.

**Filtrage** (`filter.ts`) :
- La saisie est mise en minuscules, découpée en mots, et la ponctuation est retirée.
- Chaque mot saisi doit être le **préfixe** d'un mot du libellé ou des alias (un mot du libellé ne sert qu'une fois).
- On garde les commandes qui ont le plus de mots appariés, puis, parmi elles, la plus grande longueur appariée.
- Debounce de 50 ms. Ce n'est pas du fuzzy matching caractère par caractère.

### 6.3 Écran de résultat (`components/pages/test/result/*`)

**Stats principales** :
- **wpm**, dans l'unité choisie ; « Infinite » si ≥ 1000. Couronne de PB : pending / normal / warning (non éligible, avec raison) / ineligible / error. Infobulle avec 2 décimales.
- **acc** ; infobulle « N correct / N incorrect ».

**Stats secondaires** :
- **test type** : mode + mode2 (ou nom du groupe de citation), langue, punctuation, numbers, blind, lazy, funboxes, expert/master, stop/delete on error.
- **other** : failed (raison), afk detected, invalid, repeated, bailed out, too short.
- **raw**.
- **characters** : `correct/incorrect/extra/missed`.
- **consistency** ; infobulle « X% (Y% key) ».
- **time** : « Ns » ; infobulle avec l'AFK. Lignes sous la valeur : « N% afk » et temps tapé aujourd'hui.
- **source** de la citation (signaler, favori et noter nécessitent un compte).
- tags, rang au classement quotidien (serveur).

**Graphique** (Chart.js, `ResultChart.tsx`) :
- Séries :
  - wpm : couleur main, épaisseur 3 ;
  - raw : pointillés [8,8] ;
  - burst : couleur sub, avec lissage optionnel ;
  - erreurs : nuage de croix (`crossRot`) en couleur error sur l'axe de droite.
- Axe x : secondes (libellés du §3.6). Axe gauche : vitesse. `startGraphsAtZero` est respecté.
- Lignes horizontales « PB » et « tag PB ».
- Infobulle par index ; le survol d'une seconde surligne les mots tapés pendant cette seconde.
- Légende cliquable (pb, raw, burst, erreurs, échelle) ; l'état est mémorisé en localStorage.

**Historique des mots** :
- Le mot tapé et son burst s'affichent au survol.
- Heatmap de burst (5 niveaux, légende « <a, a-b, … n+ »).
- Boutons copier les mots, les mots ratés, les mots lents.

**Replay** : lecture/pause ; cliquer sur une lettre y fait sauter le replay.

**Boutons** : Next test, Repeat test, Practice words (modal missed off/words/biwords + slow on/off), Toggle words history, Watch replay, capture d'écran (Shift = télécharger).

Réglage caché « glarses mode » : n'affiche que les boutons.

### 6.4 Page settings (`components/pages/settings/SettingsPage.tsx`)
En tête de page : navigation rapide par section, champ de recherche, sections repliables.

Ordre des sections :
- **behavior** : Tags, Presets et resultSaving (avec compte), puis les clés du §1.2 ;
- **input** ;
- **sound** (cliquer une option joue un aperçu) ;
- **caret** ;
- **appearance** (les réglages du keymap n'apparaissent que si le keymap est activé) ;
- **theme** (filtres de fond seulement si un fond est défini ; sélecteur de thème avec thèmes custom) ;
- **hide elements** ;
- **danger zone** :
  - import/export des réglages (JSON) ;
  - préférences cookies ;
  - « the rest » : message sarcastique, glarses mode, burst lissé, limite de FPS des animations ;
  - **reset settings** (confirmation ; les tags et presets sont conservés).

`showPb`, `monkey` et `monkeyPowerLevel` ne sont réglables que via la palette.

### 6.5 Page compte / stats (`components/pages/account/*`, `collections/results.ts`)
- **Stats agrégées** (`TestStats.tsx`) :
  - estimation des mots tapés = Σ round(wpm/60 × durée) ;
  - tests démarrés (= terminés + restarts) et terminés (% de tests terminés, restarts par test) ;
  - temps de frappe ;
  - plus haut / moyenne / moyenne des 10 derniers pour wpm, raw, acc et consistency.
- **Graphique d'historique** : points de vitesse, courbe en escalier des PB successifs, précision (triangles, axe gauche), moyennes mobiles sur 10 et sur 100 (bascules = config `accountChart`). Tendance « Speed change per hour spent typing ». Cliquer un point mène à la ligne du tableau.
- **Histogramme** : nombre de tests par tranche de vitesse.
- **Activité quotidienne** :
  - barres : minutes de frappe par jour ; courbe : vitesse moyenne ;
  - infobulle : tests, restarts par test, meilleure vitesse et moyennes.
- **Calendrier d'activité** (`elements/test-activity-calendar.ts`, `components/pages/profile/ActivityCalendar.tsx`) :
  - 52 semaines, ou une année au choix ;
  - 5 niveaux, seuils à 0,5×, 1× et 1,5× la moyenne tronquée à 10 % ;
  - « N tests on {date} ».
- **Cartes PB** (time 15/30/60/120, words 10/25/50/100) et tableau complet des PB (mode2, vitesse/raw, acc/consistency, difficulté, langue, ponctuation, nombres, lazy, date).
- **Filtres** : date, difficulté, PB, mode, durée/mots/longueur de citation, ponctuation, nombres, tags, funbox, langue ; presets de filtres.
- **Tableau des résultats** (triable) : PB, vitesse, raw, acc, consistency, caractères, mode, infos (icônes), tags, date. Export CSV.
- **Faisable en local** : tout ce qui précède, recalculé depuis l'historique des résultats.
- **Uniquement serveur** : classements, XP/niveau, streaks, badges, notes des citations, profil public.

### 6.6 Raccourcis clavier

| Touche | Effet | Source |
|---|---|---|
| `Tab` puis `Entrée` (par défaut, sans quickRestart) | Le focus va sur le bouton restart, puis Entrée relance. La barre d'aide affiche « tab > enter ». | `components/layout/footer/Keytips.tsx` |
| quickRestart `tab` / `esc` / `enter` | Restart immédiat ; depuis une autre page, retour à la page test ; depuis le résultat, test suivant. Shift + la touche est requis si le texte contient des tabulations (tab) ou des retours à la ligne (enter), et pour les tests longs. | `input/hotkeys/quickrestart.ts`, `utils/quick-restart.ts` |
| `Esc` (ou `Tab` si quickRestart=esc) | Palette de commandes | `states/hotkeys.ts` |
| `Ctrl/Cmd+Shift+P` | Palette de commandes | `input/hotkeys/commandline.ts` |
| `Shift+Entrée` | Termine le test zen. Tests longs : double appui en moins de 200 ms = bail out. Texte long custom : sauvegarde la progression. | `input/handlers/keydown.ts` |
| `Tab` pendant la frappe | Insère `\t` si le texte en contient (langues code) | — |
| `Ctrl/Alt+Backspace` | Efface le mot (`deleteWordBackward` natif du navigateur) | `input/handlers/delete.ts` |
| Toute touche (hors Entrée, Espace, Échap, Tab, modificateurs) quand le test n'a pas le focus | Redonne le focus. La touche est avalée si l'avertissement hors focus est actif. | `event-handlers/global.ts` |
| Konami ↑↑↓↓←→←→BA | Ouvre keymash.io | `input/hotkeys/konami.ts` |

Il n'y a pas d'autre raccourci dédié sur l'écran de résultat.

### 6.7 Notifications (`states/notifications.ts`, `components/layout/overlays/Notifications.tsx`)
- Niveaux : success, notice, error (titres par défaut « Success », « Notice », « Error »).
- Durée par défaut : 3000 ms. Les erreurs restent jusqu'au clic (durée 0).
- Option `important` : sans elle, la notification est masquée pendant le focus mode.
- Pile en haut à droite (350 px), la plus récente en premier ; clic pour fermer ; bouton « Clear all » ; historique des 25 dernières.
- Bannières et PSA (serveur) : `states/banners.ts`, `elements/psa.tsx`.
- Avertissements :
  - Caps Lock : pastille « Caps Lock » au-dessus des mots ;
  - hors focus : « Click here or press any key to focus ».
- Messages de fin de test : voir §3.7. Autres messages :
  - « Quick restart disabled in long tests… » (4 s) ;
  - « No quit funbox is active… » ;
  - « Repeat test disabled in zen mode » ;
  - « You haven't missed any words » ;
  - message sarcastique « Nice » (15 s).

---

## 7. Animations et affichage live

Chemins relatifs à `frontend/src/`. Les animations JS passent par animejs 4.2.2 ; si aucun easing n'est indiqué, c'est la valeur par défaut de la lib (`out(2)`). `applyReducedMotion(t)` renvoie 0 si l'utilisateur a demandé moins d'animations (`ts/utils/misc.ts` l.504).

### 7.1 Caret (`ts/elements/caret.ts`, `ts/test/caret.ts`, `styles/caret.scss`, `styles/animations.scss`)
- **Smooth caret** (`elements/caret.ts` l.250-259) :

  | Réglage | Durée |
  |---|---|
  | off | 0 ms (pas d'animation) |
  | slow | 150 ms |
  | medium | 100 ms |
  | fast | 85 ms |

  Easing `inOut(1.25)`. Les propriétés animées sont left/top, plus la largeur pour les styles pleine largeur.
- **Position** : le caret est placé avant la lettre cible ; en fin de mot, après la dernière lettre. Le style underline se cale sous la lettre.
- **Clignotement** : `caretFlashSmooth 1s infinite` (opacité 0 → 1 → 0). Si smooth caret est sur off, c'est `caretFlashHard` (allumé de 0 à 50 %, éteint de 51 à 100 %). Le clignotement s'arrête (opacité 1) à la première frappe et pendant tout le focus mode. Il reprend à la sortie du focus (souris bougée) ou quand le caret est réaffiché.
- **Styles** (hauteur 1.2em, couleur `--caret-color`) :

  | Style | Rendu |
  |---|---|
  | default | barre de 0.1em |
  | block | 0.5em, derrière le texte, largeur de la lettre |
  | outline | bloc avec bordure 0.05em, sans clignotement |
  | underline | 0.1em de haut, largeur de la lettre |
  | carrot / banana / monkey | images |
  | off | invisible |

### 7.2 Pace caret (`ts/test/pace-caret.ts`)
- Couleur `--sub-color`, opacité 0,5, ne clignote pas. Style donné par `paceCaretStyle`.
- Vitesse (wpm) :

  | Source | Valeur |
  |---|---|
  | pb | PB local pour la même config |
  | tagPb | PB des tags actifs |
  | average | moyenne des 10 derniers tests, arrondie |
  | daily | meilleur des dernières 24 h |
  | custom | `paceCaretCustomSpeed` |
  | last / repeat | wpm du dernier test |

  Le pace caret est désactivé si la vitesse est inférieure à 1.
- Pas : **12000 / wpm ms par caractère**. Une boucle `setTimeout` avance d'un caractère à chaque pas, avec animation linéaire. La durée est recalée sur un planning absolu, sans dérive.
- L'espace compte pour un caractère.
- Quand on valide un mot faux, le pace caret prend une avance égale à la longueur du mot. Si on corrige ce mot ensuite, l'avance est retirée. Ce mécanisme est inactif en blind mode.
- Le pace caret disparaît quand il n'y a plus de mots.
- `repeatedPace` : lors d'une répétition, `lastTestWpm` n'est remplacé que par un wpm plus élevé.

### 7.3 Lignes, tape mode, rendu des lettres (`ts/test/test-ui.ts`, `styles/test.scss`)
- **Fenêtre de 3 lignes** (`updateWordsWrapperHeight`, l.598-661). En zen : 2 lignes. `showAllLines` n'a d'effet que sur les tests non chronométrés.
- **Saut de ligne** (`lineJump`, l.1155-1224) :
  - le premier changement de ligne ne fait pas défiler ;
  - ensuite, chaque passage à la 3e ligne retire la ligne du haut, ce qui garde le caret sur la ligne du milieu ;
  - `smoothLineScroll` anime `marginTop` sur 125 ms ; sinon le saut est instantané.
- **Tape mode** (`scrollTape`, l.936-1139) :
  - `marge = largeur × tapeMargin/100 − (largeur des mots précédents + largeur des lettres tapées du mot courant)` ;
  - en mode `word`, le terme « lettres tapées » vaut 0, donc le texte défile une fois par mot ;
  - animation de 125 ms en `inOut(1.25)` (0 ms sans smoothLineScroll) ;
  - le caret principal reste fixe ; les mots sortis du cadre sont supprimés ;
  - fondu horizontal aux bords (masque : 0 à 1 %, 1 de 10 à 90 %, 0 à 99 %).
- **Couleurs des lettres** :

  | Mode | Correcte | Non tapée | Incorrecte | Extra |
  |---|---|---|---|---|
  | normal | text | sub | error | errorExtra |
  | `flipTestColors` | sub | text | error | errorExtra |
  | `colorfulMode` | main | sub | colorfulError | colorfulErrorExtra |
  | flip + colorful | sub | main | (inchangé) | (inchangé) |

- **Mot faux** validé : souligné 2px `--error-color`. Lettre corrigée : soulignement pointillé `main`. Tab/retour à la ligne : glyphes à 20 % d'opacité.
- **highlightMode** :
  - `letter` : coloration lettre par lettre (comportement par défaut) ;
  - `off` : les lettres correctes gardent la couleur « non tapée » ;
  - `word` / `next_*` : seul le mot actif (plus 1, 2 ou 3 mots suivants) est en couleur « correcte », et le mot entier passe en couleur d'erreur s'il contient une faute.
- **typedEffect** :
  - `keep` : rien ne change ;
  - `hide` : opacité 0 ;
  - `fade` : `fadeOut 250ms ease-in` ;
  - `dots` : les lettres rétrécissent à 0,4 en 200 ms, puis un point de 1em apparaît (100 ms, après 100 ms de délai). Le point est rouge en cas d'erreur.
- **indicateTypos** :
  - `replace` : affiche le caractère tapé à la place (espace affiché `_`) ;
  - `below` : rappel sous le mot (0.75em, opacité 0,5) ;
  - `both` : remplacement plus lettre attendue en dessous.
- **Blind mode** : pas d'extra, couleur correcte partout, pas de soulignement d'erreur, pas de son d'erreur.

### 7.4 Fondus et transitions
- **Restart** : le test (ou le résultat) fond à 0 en 125 ms, puis réapparaît en 125 ms (`test-ui.ts` l.1495-1519).
- **Fin de test** : le test fond en 125 ms. Un indicateur de chargement n'apparaît qu'après 0,5 s. Le résultat fond de 0 à 1 en 125 ms.
- **Changement de page** : 125 ms de fondu sortant puis 125 ms de fondu entrant (`controllers/page-controller.ts`).
- **Focus mode** (`ts/test/focus.ts`) :
  - activé au démarrage et à chaque frappe ;
  - masque pied de page, astuces clavier, nav, barre de config, notices de mode et notifications non importantes (opacité 0, 125-250 ms), ainsi que le curseur souris ;
  - désactivé si la souris bouge de plus de 3px, à la fin, au restart et à l'ouverture de la palette.
- **Hors focus** : après 1 s sans focus, les mots passent à opacité 0,25 avec `blur(4px)` (transition 0,25 s) et le message « Click here or press any key to focus » s'affiche. Le retour est instantané.

### 7.5 Stats live (`ts/states/live-stats.ts`, `ts/components/pages/test/live-stats/*`)
- **Visibilité** : seulement pendant le test et en focus, avec un fondu de 125 ms.
- **Mise à jour** :
  - wpm, raw, acc et secondes : à chaque tick de 1 s ;
  - acc : aussi à chaque frappe ;
  - burst : à chaque validation de mot.
- **Formats** :
  - vitesse : entier dans l'unité choisie (raw en blind mode) ;
  - précision : `floor(acc)%`, toujours 100 % en blind mode ;
  - burst : entier.
- **Style `text`** : très grand texte en filigrane, centré sous les mots (4-10 rem).
- **Style `mini`** : petite ligne juste au-dessus des mots, alignée à gauche, ou sur `tapeMargin` en tape mode.
- **Timer / progression** :
  - mode time : temps restant ;
  - mode infini : temps écoulé ;
  - words / quote / custom : `"index/total"` (ex. `12/50`) ;
  - zen : l'index seul.
- **Variantes `flash_*`** : le timer n'est visible que pendant le tick où `(limite − secondes) % 15 == 0`.
- **Barre (`bar`)** :
  - fixée en haut de l'écran, 0,5rem de haut ;
  - mode time : largeur `100 − (s+1)/limite × 100` %, animée sur 1000 ms linéaire ;
  - autres modes : `floor(mots/total × 100)` % en 250 ms, puis 100 % en 125 ms à la fin.
- **Couleur et opacité** : `timerColor` (black, sub, text, main) et `timerOpacity` s'appliquent à toutes les stats live.

### 7.6 Autres effets
- **Confettis** sur un nouveau PB (125 ms de rafales).
- **Monkey** :
  - 4 images selon les mains (gauche, droite, aucune, les deux) ;
  - secousse de ±2px, dont la vitesse suit le wpm entre 130 et 180 (rien sous 130 wpm).
- **monkeyPowerLevel** (`ts/elements/monkey-power.ts`) :
  - particules à chaque frappe : 6 à 9, vitesse ±1500 px/s, gravité 1000 ;
  - couleurs aléatoires aux niveaux 2 et 4 ;
  - secousse de la page de 0 à 10 px aux niveaux 3 et 4, réinitialisée après 2 s d'inactivité.
- **Heatmap de burst** (résultat) : 5 couleurs, de colorfulError à main en passant par des mélanges avec text, avec seuils aux percentiles 15/35/65/85 des bursts. Les mots non atteints sont en sub.
- **Keymap** :
  - `react` : la touche pressée flashe `main` (juste) ou `error` (faux), puis revient en 250 ms ;
  - `next` : la prochaine touche est surlignée.
- **Compte à rebours layoutfluid** : « <Layout> in: Ns » à 3, 2 puis 1 s avant le changement.
- Aucune animation de secousse sur les erreurs de frappe.
- **Sons** : déclenchés de façon synchrone dans le handler de saisie ; son d'erreur pour une frappe fausse ; avertissement de fin sur le tick ; « fart-reverb » à la fin si le son de clic 16 est actif.

---

## 8. Ce qui n'a pas été trouvé, ou dépend du serveur ou du réseau
- Il n'y a pas de fichiers JSON de challenges (les challenges sont en TypeScript), pas de champ `ligatures` dans les langues, et pas de type de layout « matrix ».
- Dépendances réseau à exclure ou à remplacer dans un clone local :
  - funboxes `poetry` (poetrydb.org) et `wikipedia` ;
  - PSA, classements, XP, streaks ;
  - notes et signalements de citations ;
  - attribution Discord des challenges.
- Pace caret `pb`, `tagPb`, `average` et `daily` : ils exigent un compte sur le site, mais sont calculables localement à partir de l'historique des résultats.
- Le CSS décoratif des thèmes (`frontend/static/themes/*.css`) et les polices web ne sont pas transposables en TUI.
