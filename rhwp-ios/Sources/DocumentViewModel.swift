import SwiftUI

/// 문서 뷰어의 뷰 모델. RhwpDocument 핸들의 수명을 뷰 생명주기와 동기화한다.
@MainActor
class DocumentViewModel: ObservableObject {
    @Published var document: RhwpDocument?
    @Published var currentPage: Int = 0
    @Published var errorMessage: String?
    @Published var isLoading: Bool = false
    @Published var filename: String = ""

    /// 페이지별 렌더 트리 캐시 (화면에 보이는 페이지만 유지)
    @Published var pageTrees: [Int: RenderNode] = [:]

    /// 문서의 총 페이지 수
    var pageCount: Int {
        document?.pageCount ?? 0
    }

    /// 특정 페이지의 크기 (포인트 단위)
    func pageSize(at page: Int) -> (width: Double, height: Double) {
        document?.pageSize(at: page) ?? (0, 0)
    }

    /// HWP/HWPX 파일 데이터를 로드한다.
    func loadDocument(data: Data, filename: String = "") {
        isLoading = true
        errorMessage = nil
        pageTrees.removeAll()

        do {
            let nameOpt = filename.isEmpty ? nil : filename
            document = try RhwpDocument(data: data, filename: nameOpt)
            self.filename = filename
            currentPage = 0
        } catch let err as RhwpError {
            errorMessage = err.errorDescription
            document = nil
        } catch {
            errorMessage = "문서를 열 수 없습니다: \(error.localizedDescription)"
            document = nil
        }

        isLoading = false
    }

    /// 번들의 샘플 파일을 로드한다.
    func loadSampleFromBundle() {
        guard let url = Bundle.main.url(forResource: "sample", withExtension: "hwpx"),
              let data = try? Data(contentsOf: url) else {
            errorMessage = "샘플 파일을 찾을 수 없습니다."
            return
        }
        loadDocument(data: data, filename: "sample.hwpx")
    }

    /// 페이지 렌더 트리를 로드한다 (onAppear에서 호출).
    func loadPage(_ page: Int) {
        guard pageTrees[page] == nil, let doc = document else { return }
        pageTrees[page] = doc.renderPageTree(at: page)
    }

    /// 페이지 렌더 트리를 해제한다 (onDisappear에서 호출, 메모리 보호).
    func unloadPage(_ page: Int) {
        pageTrees.removeValue(forKey: page)
    }
}
