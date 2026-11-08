"""Converte un PDF con Docling e salva la struttura come DoclingDocument JSON (REQ-048).

Uso (lo lancia doc-ripper con il Python di dist\\docling\\):
    python convert_pdf.py --pdf <file.pdf> --out <cartella>

Il PDF si converte a blocchi di BLOCK pagine con lo stesso converter (REQ-054): per ogni blocco scrive
<cartella>\\blocco-NNNN.json (NNNN = prima pagina del blocco) e, su stdout, la riga "pagina X/Y" con l'ultima
pagina elaborata e il totale. Le immagini vanno in <cartella>\\immagini\\, richiamate dal JSON con percorsi
relativi. I modelli si leggono da models\\ accanto allo script, senza accesso alla rete (REQ-053).
In caso di errore termina con codice 1 e il motivo su stderr.
"""
from __future__ import annotations

import argparse
import os
import shutil
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
MODELS = HERE / "models"
BLOCK = 10


def convert(pdf: Path, out: Path, models: Path) -> None:
    os.environ["HF_HUB_OFFLINE"] = "1"
    import pypdfium2
    from docling.datamodel.base_models import InputFormat
    from docling.datamodel.pipeline_options import PdfPipelineOptions, TableFormerMode
    from docling.document_converter import DocumentConverter, PdfFormatOption
    from docling_core.types.doc import ImageRefMode

    options = PdfPipelineOptions(artifacts_path=str(models))
    options.do_ocr = False  # OCR rinviato (REQ-051)
    options.do_table_structure = True
    options.table_structure_options.mode = TableFormerMode.ACCURATE
    options.table_structure_options.do_cell_matching = True
    options.generate_picture_images = True
    options.images_scale = 2.0
    converter = DocumentConverter(format_options={InputFormat.PDF: PdfFormatOption(pipeline_options=options)})

    with pypdfium2.PdfDocument(str(pdf)) as document:
        total = len(document)
    # I blocchi di un'esecuzione precedente non devono mescolarsi a quelli nuovi.
    if out.exists():
        for old in out.glob("blocco-*.json"):
            old.unlink()
        shutil.rmtree(out / "immagini", ignore_errors=True)
    out.mkdir(parents=True, exist_ok=True)
    for first in range(1, total + 1, BLOCK):
        last = min(first + BLOCK - 1, total)
        block = converter.convert(pdf, page_range=(first, last)).document
        # Cartella delle immagini relativa: gli uri restano relativi a <cartella> anche se viene spostata.
        block.save_as_json(out / f"blocco-{first:04d}.json", artifacts_dir=Path("immagini"),
                           image_mode=ImageRefMode.REFERENCED)
        print(f"pagina {last}/{total}", flush=True)


def main() -> None:
    sys.stdout.reconfigure(encoding="utf-8")
    sys.stderr.reconfigure(encoding="utf-8")
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--pdf", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--models", type=Path, default=MODELS, help="cartella dei modelli (predefinita: models\\ accanto allo script)")
    args = parser.parse_args()
    try:
        convert(args.pdf, args.out, args.models)
    except Exception as error:  # qualsiasi errore di Docling: doc-ripper ripiega su PDFium (REQ-070)
        print(f"{type(error).__name__}: {error}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
