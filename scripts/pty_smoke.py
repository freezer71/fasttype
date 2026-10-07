"""Lance fasttype dans un pseudo-terminal, tape un test custom complet, quitte.
Usage : python3 -I scripts/pty_smoke.py <binaire> <dossier HOME temporaire> [--perf] [--kitty] [--sized] [--sigterm]
  --kitty    le terminal répond OK à la sonde graphique Kitty (cases de 10 × 20 pixels)
  --sized    le terminal agrandit le texte (OSC 66, Kitty ≥ 0.40)
  --sigterm  quitte par SIGTERM au lieu de Ctrl+C"""
import fcntl, os, pty, re, select, signal, struct, sys, termios, time

binary, home = sys.argv[1], sys.argv[2]
flags = sys.argv[3:]
perf, kitty, sigterm = "--perf" in flags, "--kitty" in flags, "--sigterm" in flags
sized = "--sized" in flags
os.makedirs(f"{home}/.config/fasttype", exist_ok=True)
with open(f"{home}/.config/fasttype/config.toml", "w") as f:
    f.write('mode = "custom"\n')

pid, fd = pty.fork()
if pid == 0:
    os.environ.update({"HOME": home, "TERM": "xterm-256color", "COLORTERM": "truecolor"})
    os.environ.pop("FASTTYPE_CARET", None)
    os.environ.pop("XDG_CONFIG_HOME", None)
    os.environ.pop("XDG_DATA_HOME", None)
    os.execv(binary, [binary] + (["--perf"] if perf else []))

# 30 lignes × 100 colonnes ; 1000 × 600 pixels → cases de 10 × 20
fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", 30, 100, 1000, 600))
out = bytearray()

def pump(seconds):
    end = time.time() + seconds
    while time.time() < end:
        r, _, _ = select.select([fd], [], [], 0.02)
        if not r:
            continue
        try:
            data = os.read(fd, 65536)
        except OSError:
            return
        if not data:
            return
        out.extend(data)
        # sonde graphique : OK seulement si l'on joue un terminal Kitty
        if kitty and b"a=q" in data:
            os.write(fd, b"\x1b_Gi=31;OK\x1b\\")
        # réponses d'un terminal sans protocole clavier Kitty
        cpr = data.count(b"\x1b[6n")
        if sized and cpr == 2 and b"\x1b]66;" in data:
            os.write(fd, b"\x1b[1;1R\x1b[1;3R")  # l'espace agrandi a avancé de 2 cases
        elif cpr:
            os.write(fd, b"\x1b[1;1R" * cpr)
        if b"\x1b[?u" in data or b"\x1b[c" in data:
            os.write(fd, b"\x1b[?62c")

def plain(b):
    t = b.decode("utf-8", "replace")
    t = re.sub(r"\x1b_G[^\x1b]*\x1b\\\\", " ", t)
    t = re.sub(r"\x1b\[[0-9;?<>]*[ A-Za-z]|\x1b\][^\x07]*\x07", " ", t)
    return re.sub(r"\s+", " ", t)

pump(1.0)
for ch in "The quick brown fox jumps over the lazy dog":
    os.write(fd, ch.encode())
    pump(0.08)
pump(0.8)
screen = plain(bytes(out))
if sigterm:
    os.kill(pid, signal.SIGTERM)
else:
    os.write(fd, b"\x03")  # Ctrl+C
pump(1.0)
_, status = os.waitpid(pid, 0)
print("exit", os.waitstatus_to_exitcode(status))
print("result screen:", "test type custom english" in screen)
print("alt screen left:", b"\x1b[?1049l" in out)
print("sync output used:", b"\x1b[?2026h" in out and b"\x1b[?2026l" in out)
print("cursor color reset:", b"\x1b]112\x07" in out)
if kitty:
    print("kitty caret placed:", b"\x1b_Ga=p," in out)
    print("kitty images deleted:", out.rstrip().find(b"\x1b_Ga=d,d=A,q=2\x1b\\") > out.find(b"\x1b_Ga=p,"))
if sized:
    print("scaled words written:", b"\x1b]66;s=2;" in out)
if perf:
    found = re.findall(r"key→flush p50 ([0-9.]+) p99 ([0-9.]+) ms · frame p99 ([0-9.]+) ms · n (\d+)", bytes(out).decode("utf-8", "replace"))
    print("perf (p50, p99, frame p99, n):", found[-1] if found else None)
