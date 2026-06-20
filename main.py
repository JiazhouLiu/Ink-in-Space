# type: ignore

import base64
from transformers import TrOCRProcessor, VisionEncoderDecoderModel
from PIL import Image
import torch
import sys

processor = TrOCRProcessor.from_pretrained("microsoft/trocr-base-handwritten")
model = VisionEncoderDecoderModel.from_pretrained("microsoft/trocr-base-handwritten")

device = "cuda" if torch.cuda.is_available() else "cpu"
model = model.to(device)


def detect(path: str) -> str:
    image = Image.open(path).convert("RGB")

    pixels = processor(images=image, return_tensors="pt").pixel_values.to(device)
    ids = model.generate(pixels, max_new_tokens=64, num_beams=4)
    text = processor.batch_decode(ids, skip_special_tokens=True)[0]

    return text


if __name__ == "__main__":
    sys.stdout.buffer.write(bytes([0]))
    sys.stdout.buffer.flush()
    sys.stdout.flush()

    while True:
        path = input()
        res = detect(path)
        sys.stdout.buffer.write(res.encode())
        sys.stdout.buffer.flush()
        sys.stdout.flush()
