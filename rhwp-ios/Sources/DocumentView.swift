import SwiftUI

/// 단일 HWP 문서를 렌더링하는 뷰.
/// 향후 탭 확장 시 탭 1개 = DocumentView 1개로 대응.
struct DocumentView: View {
    @ObservedObject var viewModel: DocumentViewModel

    var body: some View {
        VStack(spacing: 0) {
            // 상단 정보 바
            headerBar

            // 문서 렌더링 영역
            if viewModel.isLoading {
                Spacer()
                ProgressView("로딩 중...")
                Spacer()
            } else if viewModel.pageCount > 0 {
                pageScrollView
            } else if let error = viewModel.errorMessage {
                errorView(error)
            } else {
                Spacer()
                Text("문서를 열어주세요.")
                    .foregroundColor(.secondary)
                Spacer()
            }
        }
    }

    // MARK: - 상단 바

    private var headerBar: some View {
        VStack(spacing: 0) {
            HStack {
                Text("알한글")
                    .font(.headline)
                    .fontWeight(.bold)
                if !viewModel.filename.isEmpty {
                    Text("— \(viewModel.filename)")
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .lineLimit(1)
                }
                Spacer()
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 10)
            .background(Color(UIColor.systemBackground))

            Divider()
        }
    }

    // MARK: - 다중 페이지 스크롤 (UIScrollView 기반 팬/줌)

    private var pageScrollView: some View {
        PagedScrollView(viewModel: viewModel)
    }

    // MARK: - 에러

    private func errorView(_ error: String) -> some View {
        VStack {
            Spacer()
            VStack(spacing: 8) {
                Image(systemName: "exclamationmark.triangle")
                    .font(.largeTitle)
                    .foregroundColor(.orange)
                Text(error)
                    .foregroundColor(.secondary)
                    .multilineTextAlignment(.center)
            }
            .padding()
            Spacer()
        }
    }
}
