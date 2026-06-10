"""Genera el icono fuente 1024x1024 (reloj, paleta del dashboard).

Uso: python3 scripts/gen_icon.py
Luego: pnpm tauri icon app-icon.png
"""

import math

from PIL import Image, ImageDraw

S = 4096  # se dibuja a 4x y se reescala a 1024 para antialiasing
C = S // 2

BG = (9, 9, 11, 255)  # zinc-950
RING = (52, 211, 153, 255)  # emerald-400
TICKS = (161, 161, 170, 255)  # zinc-400
HAND_H = (228, 228, 231, 255)  # zinc-100, hora
HAND_M = (52, 211, 153, 255)  # emerald, minutos
HAND_S = (251, 191, 36, 255)  # amber-400, segundos
HUB = (228, 228, 231, 255)

img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
d = ImageDraw.Draw(img)

# Fondo: cuadrado redondeado estilo macOS (~22.5% de radio)
radius = int(S * 0.225)
d.rounded_rectangle([0, 0, S - 1, S - 1], radius=radius, fill=BG)

# Esfera
ring_r = int(S * 0.36)
ring_w = int(S * 0.035)
d.ellipse(
    [C - ring_r, C - ring_r, C + ring_r, C + ring_r],
    outline=RING,
    width=ring_w,
)

# Marcas horarias (12, sin números)
for hour in range(12):
    ang = math.radians(hour * 30 - 90)
    outer = ring_r - int(ring_w * 1.6)
    is_quarter = hour % 3 == 0
    inner = outer - int(S * (0.045 if is_quarter else 0.025))
    w = int(S * (0.012 if is_quarter else 0.007))
    x1, y1 = C + inner * math.cos(ang), C + inner * math.sin(ang)
    x2, y2 = C + outer * math.cos(ang), C + outer * math.sin(ang)
    d.line([x1, y1, x2, y2], fill=TICKS, width=w)


def hand(angle_deg: float, length: float, width: float, color):
    ang = math.radians(angle_deg - 90)
    x = C + int(ring_r * length * math.cos(ang))
    y = C + int(ring_r * length * math.sin(ang))
    d.line([C, C, x, y], fill=color, width=int(S * width))


# 10:09:35 — composición clásica de relojería, manos abiertas
hand(10 * 30 + 9 * 0.5, 0.52, 0.022, HAND_H)
hand(9 * 6, 0.74, 0.016, HAND_M)
hand(35 * 6, 0.80, 0.007, HAND_S)

# Eje central
hub_r = int(S * 0.028)
d.ellipse([C - hub_r, C - hub_r, C + hub_r, C + hub_r], fill=HUB)

img.resize((1024, 1024), Image.LANCZOS).save("app-icon.png")
print("app-icon.png generado")
