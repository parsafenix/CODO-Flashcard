import { render, screen } from "@testing-library/react";
import { ImportHelpModal } from "../../src/features/import/ImportHelpModal";

describe("ImportHelpModal", () => {
  it("renders the beginner-friendly TXT import guide", () => {
    render(<ImportHelpModal open={true} onClose={() => undefined} />);

    expect(screen.getByText("Import guide")).toBeInTheDocument();
    expect(screen.getByText(/Each line creates one card/i)).toBeInTheDocument();
    expect(screen.getByText(/The file should be saved as UTF-8 text/i)).toBeInTheDocument();
    expect(screen.getAllByText(/Persian \| English \| Italian/i).length).toBeGreaterThan(0);
    expect(screen.getByRole("button", { name: /Copy example/i })).toBeInTheDocument();
  });
});
