# Bundled OCR models (PaddleOCR PP-OCRv5 mobile)

On-device OCR uses PaddleOCR PP-OCRv5 **mobile** ONNX models, run by the
vendored `ppocr-sdk` (see `app/libs/ppocr-sdk-release.aar`). Fully offline.

    det/inference.onnx     PP-OCRv5_mobile detection  (~4.6 MB)
    rec/inference.onnx     PP-OCRv5_mobile recognition (~15.8 MB)
    rec/inference.yml      recognition char dict + postprocess config

These are committed so a fresh checkout builds a working OCR APK. Re-fetch /
upgrade them from the official releases:

    https://huggingface.co/PaddlePaddle/PP-OCRv5_mobile_det_onnx
    https://huggingface.co/PaddlePaddle/PP-OCRv5_mobile_rec_onnx
    # or BOS:
    https://paddle-model-ecology.bj.bcebos.com/paddlex/official_inference_model/paddle3.0.0/PP-OCRv5_mobile_det_onnx_infer.tar
    https://paddle-model-ecology.bj.bcebos.com/paddlex/official_inference_model/paddle3.0.0/PP-OCRv5_mobile_rec_onnx_infer.tar

The SDK loads them from these default asset paths (`models/det/inference.onnx`,
`models/rec/inference.onnx`, `models/rec/inference.yml`) — see
`OcrViewModel`/`PaddleOCR.create`.

Models and the ppocr-sdk are Apache-2.0, derived from PaddlePaddle/PaddleOCR
(`deploy/ppocr-android`). Multi-language OCR is available by swapping in other
PP-OCRv5 rec models + dict (deferred to v1.2).
