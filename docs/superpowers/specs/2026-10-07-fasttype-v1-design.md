# fasttype v1 — conception

Date : 2026-10-07
Statut : à relire

## 1. Objectif

fasttype est un clone de [monkeytype.com](https://monkeytype.com) pour le terminal, écrit en Rust. Il fonctionne entièrement en local. Le but est de retrouver dans le terminal l'interface, les options, les animations et surtout **la sensation de fluidité** du site.

### Ce que l'utilisateur a demandé
- Toutes les options et tout ce que propose le site, avec la même interface et les mêmes animations, en Rust.
- Tout en local : historique, records et stats sur la machine. Pas de compte ni de serveur.
- Réutiliser les données de Monkeytype (langues, citations, thèmes) ; le projet est donc sous **GPL-3.0**.
- Fidélité maximale sur les terminaux modernes, avec un rendu de secours propre ailleurs.
- **La fluidité est une exigence fondatrice** : la sensation de frappe doit être aussi plaisante que sur le site.
- Toutes les langues sont embarquées, y compris les listes géantes.
- Livraison par étapes : d'abord une v1 centrée sur le cœur, utilisable au quotidien.

### Hypothèses (à corriger si besoin)
- Les terminaux cibles sont Kitty, Ghostty, WezTerm, iTerm2, Alacritty et Terminal.app, sous macOS et Linux. Windows Terminal doit fonctionner mais n'est pas testé en priorité.
- Les fichiers de config et de données suivent les chemins XDG (`~/.config`, `~/.local/share`), y compris sous macOS.
- La référence de comportement est le dépôt `monkeytypegame/monkeytype`, au commit `574d819` (2026-10-06).

### Critères de réussite de la v1
1. Un test time 30 s en `english`, du lancement de la commande à l'écran de résultat, se comporte comme sur le site : mêmes mots possibles, mêmes couleurs, même défilement et mêmes chiffres (wpm, raw, acc, consistency) pour une même séquence de frappes.
2. Latence entre la touche reçue et l'image envoyée au terminal : **p99 < 2 ms**, mesurée par `--perf` et par les benchmarks.
3. Aucun scintillement visible sur les terminaux qui supportent la sortie synchronisée.
4. Le caret glisse au pixel près sur Kitty, Ghostty et WezTerm.
5. Les 187 thèmes, les 446 langues et les 15 828 citations sont disponibles hors ligne.

## 2. Découpage du projet complet

| Étape | Contenu |
|---|---|
| **v1 (cette spec)** | Moteur de frappe, modes time/words/quote/zen/custom simple, écran de test, caret fluide, résultat avec graphique, palette de commandes, thèmes, langues, citations, historique et PB locaux. |
| v2 | Le reste des réglages Behavior/Input (difficulté, stop/delete on error, confidence, freedom, min speed/acc/burst, lazy, blind, highlightMode, typedEffect, indicateTypos, tape mode), pace caret, practice words, sons. |
| v3 | Funboxes (sauf `poetry` et `wikipedia`, qui dépendent du réseau) et challenges. |
| v4 | Page settings avec recherche, page stats/compte (graphiques d'historique, calendrier d'activité, tableaux et filtres, export CSV), keymap, historique des mots et heatmap, replay, monkey, confettis. |

Chaque étape aura sa propre spec, son plan et son implémentation. Les fondations de la v1 sont conçues pour accueillir les étapes suivantes sans réécriture : la config contient toutes les clés dès la v1, et le journal d'événements suffit pour le replay et la heatmap.

L'inventaire complet de Monkeytype (104 clés de config, formules, funboxes, animations) sert de référence. Il sera copié dans `docs/reference/monkeytype-inventory.md`.

## 3. Architecture

L'approche retenue est **`ratatui` + `crossterm` + un moteur d'animation maison**, avec le caret au pixel près grâce au protocole graphique Kitty.

C'est un workspace Cargo de quatre crates :

```
fasttype/
├── crates/
│   ├── fasttype-core    logique pure : génération, session, stats (aucune E/S)
│   ├── fasttype-data    données embarquées (langues, citations, thèmes)
│   ├── fasttype-store   config, historique, PB sur disque
│   └── fasttype-tui     interface, animations, binaire `fasttype`
├── xtask/               récupération et préparation des données Monkeytype
├── assets/              données compressées (générées par xtask, versionnées)
└── docs/
```

Dépendances entre crates : `tui → core, data, store`, `store → core` et `data → core` (pour `QuoteFile` et `remove_language_size`). `core` ne dépend de rien d'autre dans le projet. `data` dépend en plus de `serde`, `serde_json` et `zstd` ; son catalogue embarqué est derrière la feature `embedded`, que `xtask` n'active pas.

## 4. `fasttype-core` : moteur de frappe et stats

### 4.1 Journal d'événements
Comme Monkeytype (`test/events/stats.ts`), la session enregistre tous les événements horodatés en millisecondes depuis le début du test :

```rust
enum TestEvent {
    Start,
    KeyDown { code, t },
    KeyUp { code, t },
    Insert { ch, word_index, char_index, correct: bool, t },
    Delete { kind: DeleteKind, t },          // char | word
    WordCommit { word_index, input, correct, t },
    TimerTick { second, t },
    End { reason: EndReason, t },
}
```

Toutes les stats sont des **fonctions pures du journal**. Le journal est pré-alloué : sa capacité est réservée au démarrage, pour éviter toute allocation pendant la frappe (voir §7).

### 4.2 Composants
| Composant | Rôle |
|---|---|
| `Clock` (trait) | Fournit `now_ms()`. `SystemClock` dans l'application, `ManualClock` dans les tests. |
| `WordGenerator` | Tirage uniforme dans la liste de la langue, avec les règles de rejet de Monkeytype : pas le même mot que l'un des 2 précédents, pas de « I » sans ponctuation, pas de symboles sans ponctuation (hors langues code), pas de chiffres sans `numbers`, jusqu'à 100 tentatives. Sans ponctuation, les mots passent en minuscules (sauf allemand, code, klingon). Ponctuation en cascade (`punctuateWord`) avec les probabilités exactes, y compris les variantes propres au français et à l'espagnol. Nombres : probabilité 0,1, de 1 à 4 chiffres, le premier non nul. La source aléatoire est injectable (`rand::Rng` avec une graine). Environ 100 mots sont générés d'avance. |
| `QuotePicker` | Choisit une citation par longueur (all, short, medium, long, thicc), en suivant les groupes de longueur définis dans le fichier de citations. |
| `CustomText` | Mode simple, repeat, shuffle ou random, avec une limite en mots, en sections ou en secondes (0 = infini), et le délimiteur `|`. |
| `TestSession` | Machine à états : `Ready → Running → Finished`. Elle applique les règles de saisie de Monkeytype (§4.3) et les règles de fin (§4.4). |
| `stats` | `count_chars`, `wpm`, `raw`, `accuracy`, `consistency` (`kogasa`), `burst`, `chart_series`, `afk`, `validity`. |
| `ResultRecord` | Résultat final sérialisable (`serde`), avec sa clé de PB. |

### 4.3 Règles de saisie (v1)
- Un espace sur un mot vide est ignoré.
- La saisie d'un mot est limitée à la longueur de la cible + 20 caractères (+ 30 en zen).
- L'espace valide le mot, même s'il est faux. Un mot faux validé est marqué et reste corrigeable.
- Backspace : on ne peut pas revenir sur un mot précédent s'il était correct. `ctrl+backspace` ou `alt+backspace` efface le mot en cours.
- `\n` n'est accepté que si le texte contient des retours à la ligne, ou en zen.
- Le test démarre à la première insertion.

### 4.4 Fin du test
- **time** : au tick où le temps écoulé atteint la durée (15, 30, 60, 120 ou durée custom ; 0 = infini).
- **words / quote / custom** : sur le dernier mot, quand le mot est exact ou validé.
- **zen** : uniquement avec `shift+enter`. Le temps mort final est retiré s'il dure moins de 7 s.
- **bail out** (depuis la palette) : le test se termine avec `bailed_out = true`, sans PB.

### 4.5 Formules (reprises à l'identique)
```
calculate_wpm(chars, s) = s <= 0 ? 0 : chars / 5 / (s / 60)
wpm   = round2(calculate_wpm(correct_word_chars, duration))
raw   = round2(calculate_wpm(all_correct + incorrect + extra, duration))
acc   = round2(correct_inputs / (correct_inputs + incorrect_inputs) * 100)   // chaque insertion, même corrigée ensuite
kogasa(c) = 100 * (1 - tanh(c + c^3/3 + c^5/5))
consistency = round2(kogasa(stddev_pop(burst_per_second) / mean(burst_per_second)))   // 0 si NaN
```
- `count_chars` suit exactement l'algorithme de Monkeytype : l'espace séparateur fait partie de la cible, et le dernier mot reçoit un crédit partiel en mode time et en bail out.
- **Graphique** : les points sont sur une grille de 1 s. On ajoute une borne finale fractionnaire si le reste dépasse 0,5 s (tests non chronométrés). Séries : wpm cumulé, burst par seconde et erreurs par seconde. Monkeytype ne garde pas le graphique des tests de plus de 122 s (limite de son serveur) ; fasttype garde la série complète en local.
- **AFK** : aucune insertion pendant les 5 dernières secondes.
- **Invalidations**, dans cet ordre : test trop court (moins d'1 s ; time < 15 ; words < 10 ; zen < 15 s), AFK, test répété, wpm ou raw hors de [0, 350] (420 pour words 10), précision < 75 %. Un résultat invalide est affiché mais pas enregistré.
- **Clé de PB** : `(mode, mode2, punctuation, numbers, language, difficulty, lazy_mode)`. Un PB est mis à jour seulement si le wpm est strictement supérieur. Les citations ne comptent pas pour les PB.

### 4.6 Restart et repeat
- **Restart** : nouveau jeu de mots. Il est refusé par le raccourci rapide pour les tests longs (words ≥ 1000 ou 0, time ≥ 900 ou 0) ; dans ce cas il faut ajouter `shift`.
- **Repeat** : les mêmes mots. Le test est marqué « repeated » et n'est pas enregistré, sauf en quote : une citation répétée reste enregistrée (sans compter pour les PB), comme sur Monkeytype. Indisponible en zen.
- **Équivalences** : comme Monkeytype (`normalizeData`), une frappe typographiquement équivalente au caractère attendu est acceptée (`'` pour `’`, `"` pour `“`, `-` pour `—`, `е` pour `ё` en russe) et toute espace Unicode vaut une espace.

## 5. `fasttype-tui` : interface et animations

### 5.1 Boucle principale et rendu
```
thread input  ──(canal borné)──▶  boucle principale
                                     │ 1. vider tous les événements en attente
                                     │ 2. les appliquer à TestSession / UI
                                     │ 3. avancer les animations à t = now
                                     │ 4. rendre (diff de cellules) + flush unique
                                     └ 5. attendre : prochain événement OU prochaine image si une animation tourne
```
- **Rendu immédiat à la frappe** : une frappe déclenche un rendu sans attendre la prochaine image d'animation.
- **Cadence des animations** : un planning absolu, sans dérive, réglable à 60, 120 ou 144 images/s (60 par défaut). Quand rien ne bouge, la boucle attend sans timeout : 0 % de CPU.
- **Sortie synchronisée** (DEC mode `2026`) autour de chaque image, quand le terminal la supporte (détection par DECRQM au démarrage).
- Chaque image est envoyée en un seul `write`, depuis un tampon réutilisé.

### 5.2 Moteur d'animation
- `Tween<T> { from, to, start_ms, duration_ms, easing }`. Les easings reprennent ceux d'anime.js : `linear`, `out(2)` (par défaut), `inOut(1.25)`.
- **Reciblage** : changer la cible d'une animation en cours la fait repartir de la valeur interpolée actuelle. Le caret ne revient jamais en arrière et ne téléporte jamais.
- Toutes les animations lisent la même `Clock`, ce qui les rend testables à un instant t précis.

### 5.3 Écrans de la v1
**Écran de test**
- En-tête : `fasttype`, puis la barre de config (`@ punctuation # numbers │ time words quote zen custom │ options du mode`). En quote, punctuation et numbers sont désactivés ; en zen, ils sont masqués.
- Badge de la langue au-dessus des mots.
- Zone de mots : 3 lignes (2 en zen), centrée, avec une largeur maximale réglable. Le défilement commence au deuxième saut de ligne et dure 125 ms (`smoothLineScroll`).
- Couleurs des lettres : `text` pour une lettre correcte, `sub` pour une lettre non tapée, `error` pour une erreur, `errorExtra` pour une lettre en trop. Un mot faux validé est souligné en couleur `error`. `flipTestColors` et `colorfulMode` sont inclus dès la v1.
- Stats live : `timerStyle` = `mini` (par défaut), `text`, `bar` ou `off`, plus live wpm, acc et burst (désactivés par défaut), mis à jour à chaque tick d'1 s. Les variantes `flash_*` sont reportées à la v2.
- Pied de page : `tab + enter - restart   esc - command line`.
- **Focus mode** : dès la première frappe, l'en-tête, la barre et le pied de page passent à l'opacité 0 en 125 ms. Une « opacité » se simule en mélangeant chaque couleur avec `bg`. Ils réapparaissent à la fin du test, à l'ouverture de la palette ou quand la souris bouge (si le terminal remonte ses mouvements).
- Avertissements « terminal trop petit » et « Caps Lock » (seulement si le protocole clavier Kitty le signale).

**Caret**
- Styles : `default` (barre), `block`, `outline`, `underline` et `off`.
- `smoothCaret` : off, slow, medium ou fast (0, 150, 100 ou 85 ms), courbe `inOut(1.25)`.
- Clignotement doux avec une période d'1 s (`caretFlashSmooth`), ou franc quand smoothCaret est sur off. Le caret ne clignote plus dès la première frappe.
- Trois niveaux de rendu, choisis par détection au démarrage ou forcés par `caretRenderer` :
  1. `kitty` : une image (rectangle de la couleur `caret`) transmise une seule fois, puis déplacée au pixel près à chaque image. La taille d'une case en pixels vient de `CSI 16 t` ou de `TIOCGWINSZ`.
  2. `subcell` : en truecolor, le caret avance par demi-case grâce à des demi-blocs, et le fond de la lettre est teinté pendant le glissement.
  3. `cell` : saut de case et clignotement.

**Écran de résultat**
- wpm et acc en grand, avec une couronne quand on bat un PB.
- Graphique en braille : wpm en couleur `main` (trait épais), raw en pointillés, burst en couleur `sub` et croix d'erreurs en couleur `error` sur l'axe de droite. L'axe des x est en secondes.
- Statistiques : test type, other (invalid, afk, repeated, too short…), raw, characters (`c/i/e/m`), consistency, time (et part d'AFK), source de la citation.
- Transitions : le test disparaît en fondu de 125 ms, puis le résultat apparaît en fondu de 125 ms. Même transition au restart.
- `tab + enter` lance le test suivant ; `esc` ouvre la palette (Next test, Repeat test).

**Palette de commandes** (`esc` ou `ctrl+shift+p`)
- Catégories : Résultat (si on est sur l'écran de résultat), Test, Behavior, Input, Caret, Appearance, Theme, Show/hide et Other. Seules les entrées implémentées en v1 apparaissent.
- Filtrage de Monkeytype : la saisie passe en minuscules et est découpée en mots ; chaque mot doit être le début d'un mot du libellé ou d'un alias. Les commandes sont classées par nombre de mots appariés, puis par longueur appariée. Il n'y a pas de fuzzy matching.
- Navigation : ↑↓, `ctrl+j/k`, `ctrl+n/p` et `tab`/`shift+tab` pour se déplacer, `entrée` pour exécuter, `esc` pour remonter d'un niveau ou fermer. Un mode saisie permet les valeurs libres (durée custom, nombre de mots, texte custom).
- Thèmes : un aperçu en direct au survol, et un retour au thème précédent si on annule.
- Entrées Other : Bail out (pour les tests longs ou zen), Export/Import settings (TOML), Quit.

**Notifications** : une pile en haut à droite. Elles disparaissent après 3 s ; les erreurs restent jusqu'à ce qu'on les ferme. Exemples : « Test invalid - too short », « Quick restart disabled in long tests ».

**Taille des mots** (ajout du 2026-10-08) : comme sur le site, les mots sont à `fontSize` fois la taille du reste de l'interface (2 par défaut, échelle entière de 1 à 4). Trois rendus, choisis au démarrage :
1. `osc66` : le protocole de texte agrandi de Kitty (≥ 0.40), avec la police du terminal.
2. `glyphs` : si le terminal affiche les images Kitty mais pas OSC 66 (Ghostty, WezTerm), chaque lettre est dessinée en image avec la police du site (Roboto Mono, embarquée, SIL OFL 1.1), transmise compressée une fois par couleur du thème, puis placée sur la grille. Seules les lettres qui changent sont replacées. Si la police n'a pas une lettre du test (CJK, hébreu…), les mots restent à la taille de base.
3. Sinon, taille de base : seul le zoom du terminal agrandit.

**Restart** (ajout du 2026-10-08, demande de l'utilisateur) : `quickRestart` vaut `tab` par défaut (le site : `off`, c'est-à-dire tab puis enter). Le nouveau test est créé tout de suite et apparaît en fondu ; les touches tapées pendant ce fondu comptent (le site les ignore 250 ms).

### 5.4 Thèmes et couleurs
- 10 couleurs par thème : `bg`, `main`, `caret`, `sub`, `subAlt`, `text`, `error`, `errorExtra`, `colorfulError`, `colorfulErrorExtra`.
- Si le terminal n'a pas le truecolor (`COLORTERM`), chaque couleur est convertie vers la plus proche des 256 couleurs (distance perceptuelle OKLab), avec un cache.
- Le fond de toute la fenêtre est peint avec `bg`.

## 6. Données et stockage

### 6.1 `xtask` et `fasttype-data`
- `cargo xtask fetch-data --rev 574d819` télécharge, au commit indiqué :
  - `frontend/static/languages/*.json` ;
  - `frontend/static/quotes/*.json` ;
  - les thèmes, extraits de `frontend/src/ts/constants/themes.ts` par un analyseur strict qui échoue sur une forme inattendue ;
  - les groupes de langues (`LanguageGroups` de `frontend/src/ts/constants/languages.ts`), écrits dans `assets/language_groups.json`.
- Il valide le JSON et produit des fichiers zstd (niveau 19, mode long) dans `assets/`, avec un `assets/manifest.toml` qui contient les SHA-256 et le commit d'origine.
- `assets/` est versionné, environ 40 Mo. Le build n'a donc pas besoin d'Internet.
- `fasttype-data` embarque tous les fichiers avec `include_bytes!`. Au démarrage, seul l'index des noms est lu. **Une langue n'est décompressée que lorsqu'on la choisit**, puis gardée en mémoire. Les grosses langues sont décompressées dans un thread d'arrière-plan pour ne pas bloquer l'interface, avec l'indication « loading… » sur le badge.
- Licence : le dépôt est sous GPL-3.0, avec un fichier `NOTICE` qui crédite Monkeytype et indique le commit d'origine.

### 6.2 `fasttype-store`
| Fichier | Contenu |
|---|---|
| `~/.config/fasttype/config.toml` | Toutes les clés de Monkeytype, avec leurs valeurs par défaut (en `snake_case`). Une clé inconnue ou invalide est ignorée et signalée par une notification ; l'application ne plante jamais. L'écriture est atomique (fichier temporaire puis `rename`). |
| `~/.local/share/fasttype/results.jsonl` | Un `ResultRecord` par ligne, ajouté à la fin du fichier en un seul `write` après le test. Une ligne illisible est ignorée et comptée. |
| `~/.local/share/fasttype/personal_bests.json` | Cache des PB, reconstructible depuis `results.jsonl` (`fasttype --rebuild-pbs`). |
| `~/.local/share/fasttype/custom_texts/` | Textes custom sauvegardés. |
| `~/.local/share/fasttype/favorite_quotes.json` | Citations favorites. |

`XDG_CONFIG_HOME` et `XDG_DATA_HOME` sont respectés.

## 7. Fluidité (exigence transversale)

| Exigence | Moyen |
|---|---|
| Latence touche → flush < 2 ms (p99) | Thread d'entrée dédié, rendu immédiat à la frappe, diff de cellules, `write` unique. |
| Pas de délai sur Échap | Protocole clavier Kitty (flags *disambiguate* et *report event types*) quand il est disponible ; sinon, un timeout Échap court (25 ms). |
| Pas de scintillement | Sortie synchronisée (mode 2026), aucun effacement d'écran entre deux images, image caret Kitty réutilisée. |
| Animations régulières | Planning d'images absolu, reciblage des tweens, `Clock` unique. |
| Pas d'à-coups pendant la frappe | Aucune allocation dans le chemin frappe → rendu (tampons, journal et mots pré-alloués), aucune E/S disque avant la fin du test. |
| Démarrage < 50 ms | Données décompressées à la demande et config lue en une passe. |
| Mesurer en continu | `fasttype --perf` (surimpression : latence p50/p99, temps d'image, images ratées) et benchmarks `criterion` (§8). |

Build de production : `lto = "fat"`, `codegen-units = 1`, `panic = "unwind"` (nécessaire pour que le hook de panique restaure le terminal).

## 8. Gestion des erreurs
- **Restauration du terminal garantie** : un garde RAII et un hook de panique quittent l'écran alternatif, réaffichent le curseur, désactivent le protocole clavier, la souris et la sortie synchronisée, et suppriment l'image Kitty.
- **Terminal trop petit** (moins de 40 × 10 environ) : un écran explicatif avec la taille minimale.
- **Fichiers illisibles** : on se replie sur les valeurs par défaut et on affiche une notification ; aucune donnée utilisateur n'est écrasée. Une config invalide est mise de côté dans `config.toml.bak`.
- **Données embarquées** : leur validité est garantie par les tests de §9. Une erreur à l'exécution déclenche une notification et un retour à `english`.

## 9. Tests
- **`core`** (TDD) : chaque formule est testée sur des journaux écrits à la main, avec des valeurs attendues calculées d'après les formules de Monkeytype. Tests de propriétés (`proptest`) : une acc entre 0 et 100, `wpm ≤ raw` quand il n'y a pas de crédit partiel. Générateur avec graine : sortie reproductible et règles de rejet respectées. Fin de test pour chaque mode.
- **`data`** : chaque langue, citation et thème embarqué se charge, et chaque couleur est valide.
- **`store`** : écriture puis relecture de l'historique ; ligne corrompue ignorée ; config inconnue ou invalide ; reconstruction des PB.
- **`tui`** : snapshots `insta` du buffer `ratatui` pour l'écran de test, le résultat, la palette et l'écran trop petit. Animations à t fixé (caret à 0, 50 et 150 ms ; reciblage en cours d'animation). Filtrage de la palette sur les exemples de Monkeytype.
- **Benchmarks** (`criterion`) : rendu d'une image en 200 × 60 sous 1 ms ; traitement frappe + rendu sous 2 ms au p99 sur le rejeu d'un journal de 10 000 frappes rapides.
- **Validation manuelle** : `--perf` dans Kitty, Ghostty, iTerm2 et Terminal.app, avec relevé des chiffres.

## 10. Hors périmètre de la v1
Tout ce qui est listé en v2, v3 et v4 au §2. De plus, de façon définitive : classements, XP, comptes, PSA et funboxes réseau. Le choix de la police est impossible dans un terminal. Les fonds d'écran custom et les filtres CSS ne sont pas transposables.
