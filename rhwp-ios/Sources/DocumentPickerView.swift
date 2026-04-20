// 파일 선택 UI — UIDocumentPickerViewController를 SwiftUI에서 사용

import SwiftUI
import UniformTypeIdentifiers

struct DocumentPickerView: UIViewControllerRepresentable {
    let onPick: (Data, String) -> Void
    let onError: ((RhwpError) -> Void)?

    init(onPick: @escaping (Data, String) -> Void, onError: ((RhwpError) -> Void)? = nil) {
        self.onPick = onPick
        self.onError = onError
    }

    func makeCoordinator() -> Coordinator {
        Coordinator(onPick: onPick, onError: onError)
    }

    func makeUIViewController(context: Context) -> UIDocumentPickerViewController {
        // HWP/HWPX + 일반 데이터 UTType
        var types: [UTType] = [.data]
        // 커스텀 UTType (시스템에 등록되지 않은 경우 .data로 폴백)
        if let hwp = UTType("com.hancom.hwp") { types.append(hwp) }
        if let hwpx = UTType("com.hancom.hwpx") { types.append(hwpx) }

        let picker = UIDocumentPickerViewController(forOpeningContentTypes: types)
        picker.delegate = context.coordinator
        picker.allowsMultipleSelection = false
        return picker
    }

    func updateUIViewController(_ uiViewController: UIDocumentPickerViewController, context: Context) {}

    class Coordinator: NSObject, UIDocumentPickerDelegate {
        let onPick: (Data, String) -> Void
        let onError: ((RhwpError) -> Void)?

        init(onPick: @escaping (Data, String) -> Void, onError: ((RhwpError) -> Void)?) {
            self.onPick = onPick
            self.onError = onError
        }

        func documentPicker(_ controller: UIDocumentPickerViewController, didPickDocumentsAt urls: [URL]) {
            guard let url = urls.first else { return }
            let filename = url.lastPathComponent

            // 보안 범위 접근 시작
            guard url.startAccessingSecurityScopedResource() else {
                onError?(.accessDenied(filename: filename))
                return
            }
            defer { url.stopAccessingSecurityScopedResource() }

            do {
                let data = try Data(contentsOf: url)
                onPick(data, filename)
            } catch {
                onError?(.fileReadFailure(filename: filename))
            }
        }
    }
}
