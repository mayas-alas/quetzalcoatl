#!/usr/bin/env python3
"""karma.py — calcula XP / nivel / badges / líder desde la TRACKING de agentA.

Determinista, solo stdlib, agnóstico de herramienta. Lee la tabla maestra
(TABLES globals) y emite la progresión en la sección "## Progresión" de
TRACKING.md o por stdout; no escribe salvo que se pida --write.

Uso:
  python karma.py            -> imprime progresión calculada en stdout.
  python karma.py --write    -> inserta/reemplaza la sección "## Progresión" de TRACKING.md.
  python karma.py --check    -> exit 0 si la sección está al día (consistent).
"""

import argparse
import re
import sys
import datetime as _dt
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent  # .AGENTS/agentA
TRACKING = ROOT / "TRACKING.md"
PROG_HEADER = "## Progresión (regenerada por python/karma.py)"

# Puntos (GAMIFIED.md): (+) por acción, (-) por anti-pattern.
XP_DONE = 40            # done firmado por grado >=2
XP_CLEAN = 20           # entrega sin correcciones
XP_EARLY = 10           # antes del time-box (en revisión; se otorga a mano)
XP_BUG_HUNTER = 15      # verificación atrapa bug real
XP_OVERWATCH = 15       # evaluador detecta bucle/overrun
XP_MENTOR = 10          # mentoría doer->sr
XP_ARCH = 20            # plan de arq aceptado
XP_UNBLOCK = 5          # bloqueo resuelto
XP_SELF_DONE = -40      # auto-firma del propio done (prohibido)
XP_LOOP_CORR = -15      # por corrección introducida (bucle)
XP_REPEAT_FIX = -20     # reintento del mismo fix

LEVELS = [  # (min_xp, title, grade)
    (0, "doer", 1),
    (100, "doer avanzado", 1),
    (250, "sr", 1),
    (500, "verificador/evaluador", 2),
    (900, "juez/arquitecto", 3),
]


def human_now():
    return _dt.datetime.now(_dt.timezone.utc).strftime("%Y-%m-%d")


def parse_rows(text):
    """Devuelve lista de dicts con las filas de las tablas del documento.

    Cada tabla es cabecera + separador (---) + filas de datos. Se procesan
    todas las tablas; solo las filas con la columna `state` participan en la
    progresión.
    """
    lines = text.splitlines()
    rows = []
    i = 0
    n = len(lines)
    while i < n:
        line = lines[i].rstrip()
        if not line.startswith("|"):
            i += 1
            continue
        header = [c.strip() for c in line.strip("|").split("|")]
        sep = lines[i + 1].rstrip() if i + 1 < n else ""
        if not sep.startswith("|"):
            i += 1
            continue
        sep_cells = [c.strip() for c in sep.strip("|").split("|")]
        if not (
            len(sep_cells) == len(header)
            and re.fullmatch(r":?-+:?", sep_cells[0] or "")
        ):
            i += 1
            continue
        i += 2
        while i < n:
            row_line = lines[i].rstrip()
            if not row_line.startswith("|"):
                break
            cells = [c.strip() for c in row_line.strip("|").split("|")]
            if re.fullmatch(r":?-+:?", cells[0] or ""):
                break
            if len(cells) == len(header):
                rows.append(dict(zip(header, cells)))
            i += 1
    return rows


def effort_hours(value):
    m = re.findall(r"(\d+(?:\.\d+)?)", value or "")
    return sum(float(x) for x in m)


def compute(tracking_text):
    rows = parse_rows(tracking_text)
    done = [r for r in rows if r.get("state") == "done"]
    score = {}  # agent_id -> dict(points, corr, done_clean)
    for r in done:
        role = r.get("role") or r.get("agent") or "?"
        s = score.setdefault(role, {"points": 0, "corr": 0, "done": 0,
                                    "clean": 0})
        corr = 0
        try:
            corr = int(float(r.get("corr") or 0))
        except ValueError:
            pass
        s["corr"] += corr
        s["done"] += 1
        pts = XP_DONE
        if corr == 0:
            pts += XP_CLEAN
            s["clean"] += 1
        pts += XP_LOOP_CORR * corr
        s["points"] += pts
        effort = effort_hours(r.get("effort"))
        s.setdefault("effort", 0.0)
        s["effort"] += effort
    return rows, score


def level_for(points):
    out = level = (0, "doer", 1)
    for lv in LEVELS:
        if points >= lv[0]:
            out = lv
    return out


def render(rows, score):
    lines = [PROG_HEADER, ""]
    if not score:
        lines.append("_Sin datos todavía (primer entregable en curso)._")
        return "\n".join(lines)
    lines.append("| role | nivel | XP | correcciones | esfuerzo(h) | streak clean | badges |")
    lines.append("|---|---|---:|---:|---:|---:|---|")
    for role in sorted(score):
        s = score[role]
        lv = level_for(s["points"])
        lines.append(
            f"| {role} | {lv[1]} (g{lvl_grade(lv)}) | {s['points']} | "
            f"{s['corr']} | {s['effort']:.1f} | {s['clean']} | {badges_for(s)} |"
        )
    return "\n".join(lines)


def lvl_grade(lv):
    return lv[2]


def badges_for(s):
    badges = []
    if s["clean"] >= 3:
        badges.append("one-shot")
    if s["done"] >= 3 and s["corr"] == 0:
        badges.append("on-time")
    return ", ".join(badges) if badges else "-"


def _replace_section(text, new_section):
    lines = text.splitlines()
    out = []
    i = 0
    while i < len(lines):
        if lines[i] == PROG_HEADER:
            while i < len(lines) and lines[i].strip() != "":
                i += 1
            # saltar dos líneas en blanco
            i += 1
            out.extend(new_section.splitlines())
            out.append("")
            out.append("")
            return "\n".join(out).rstrip() + "\n"
        out.append(lines[i])
        i += 1
    out.extend(["", new_section])
    return "\n".join(out).rstrip() + "\n"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--write", action="store_true", help="actualizar TRACKING.md")
    ap.add_argument("--check", action="store_true", help="verificar coherencia")
    args = ap.parse_args()
    if not TRACKING.exists():
        print(f"no se encuentra {TRACKING}", file=sys.stderr)
        return 2
    text = TRACKING.read_text(encoding="utf-8")
    rows, score = compute(text)
    section = render(rows, score)
    if args.write:
        new = _replace_section(text, section)
        TRACKING.write_text(new, encoding="utf-8")
        print(f"TRACKING.md actualizado ({human_now()})")
        return 0
    if args.check:
        current = text.split(PROG_HEADER)[-1].strip()
        if current.strip() == section.split(PROG_HEADER, 1)[-1].strip():
            print("coherente")
            return 0
        print("desactualizado; ejecuta: python karma.py --write",
              file=sys.stderr)
        return 1
    print(section)
    return 0


if __name__ == "__main__":
    sys.exit(main())