import { render, screen } from "@testing-library/react";
import { FieldText } from "../../src/components/ui/FieldText";

describe("FieldText", () => {
  it("renders text with automatic bidi handling", () => {
    render(<FieldText value="سلام Hello" />);
    expect(screen.getByText("سلام Hello")).toHaveAttribute("dir", "auto");
  });

  it("renders a fallback for empty values", () => {
    render(<FieldText value="" />);
    expect(screen.getByText("-")).toBeInTheDocument();
  });
});

