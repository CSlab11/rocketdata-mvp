"""Scarica UNA VOLTA il modello di embedding nella cartella locale (richiede internet).
Dopo questo passo tutto il sistema funziona offline.

    python setup_model.py
"""
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parent
cfg = yaml.safe_load(open(ROOT / "config.yaml", encoding="utf-8"))["modello"]
dest = ROOT / cfg["cartella"]

from sentence_transformers import SentenceTransformer  # noqa: E402

print(f"Scarico {cfg['nome_hf']} in {dest} ...")
SentenceTransformer(cfg["nome_hf"]).save(str(dest))
print("Fatto. Da ora il sistema lavora offline.")
