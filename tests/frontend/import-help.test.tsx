import { render, screen } from "@testing-library/react";
import { ImportHelpModal } from "../../src/features/import/ImportHelpModal";
import { I18nProvider } from "../../src/lib/i18n";

describe("ImportHelpModal", () => {
  it("renders the beginner-friendly TXT import guide", () => {
    render(
      <I18nProvider language="en">
        <ImportHelpModal open={true} onClose={() => undefined} />
      </I18nProvider>
    );

    expect(screen.getByText("Import guide")).toBeInTheDocument();
    expect(screen.getByText(/Each non-empty row creates one card/i)).toBeInTheDocument();
    expect(screen.getByText(/The file must be saved as UTF-8 text/i)).toBeInTheDocument();
    expect(screen.getByText(/Only fields marked as required in the target deck/i)).toBeInTheDocument();
    expect(screen.getAllByText(/Persian \| English/i).length).toBeGreaterThan(0);
    expect(screen.getByRole("button", { name: /Copy example/i })).toBeInTheDocument();
  });
});
