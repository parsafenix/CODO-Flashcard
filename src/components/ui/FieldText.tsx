interface FieldTextProps {
  value: string | null | undefined;
  className?: string;
}

export function FieldText({ value, className = "" }: FieldTextProps) {
  return (
    <span dir="auto" className={`field-text ${className}`.trim()}>
      {value?.trim() ? value : "-"}
    </span>
  );
}
