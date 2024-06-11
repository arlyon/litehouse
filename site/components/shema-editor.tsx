"use client";

import Ajv from "ajv";
import { useEffect, useState } from "react";
import { useForm } from "react-hook-form";
import { toast } from "sonner";
import z from "zod";
import { CopyBox } from "./copy-box";
import { Input } from "./ui/input";
import { useRouter } from "next/navigation";
import { useHashState } from "@/hooks/use-hash";

const configSchema = (name: string) =>
  z.object({
    type: z.literal("object"),
    required: z.array(z.string()),
    properties: z.object({
      config: z.object({
        properties: z.record(z.any()),
        required: z.array(z.string()),
        type: z.literal("object"),
      }),
      plugin: z.object({
        const: z.literal(name),
      }),
    }),
  });

export const SchemaEditor = ({ id, schema: schemaString }) => {
  const page = useRouter();
  const [hash, setHash] = useHashState({});

  let defaultData = {};
  if (hash) {
    try {
      defaultData = JSON.parse(window.atob(hash)).instance;
    } catch (e) {
      setHash(null);
    }
  }

  const form = useForm({
    defaultValues: {
      config: defaultData,
      plugin: id,
    },
  });

  const [text, setText] = useState(
    JSON.stringify(
      {
        $schema: "./schema.json",
        plugins: {
          instance: form.getValues().config,
        },
        imports: [id],
      },
      null,
      2,
    ),
  );
  const [addCommand, setAddCommand] = useState<string | null>(null);

  useEffect(() => {
    if (addCommand) {
      setHash(addCommand);
    }
  }, [addCommand]);

  useEffect(() => {
    if (hash) {
      try {
        const data = JSON.parse(window.atob(hash));
        const dataInner = data.instance;
        form.setValue("config", dataInner);
      } catch (e) {
        // no-op
      }
    }
  }, [hash]);

  const schemaData = JSON.parse(schemaString);
  const schema = configSchema(id).safeParse(schemaData);
  if (!schema.success) {
    return <div>Invalid schema</div>;
  }

  const nonConstFields = schema.data.properties.config.properties;

  form.watch((data, { name, type }) => {
    setText(
      JSON.stringify(
        {
          $schema: "./schema.json",
          plugins: {
            instance: data,
          },
          imports: [id],
        },
        null,
        2,
      ),
    );

    const conf = JSON.stringify({
      instance: data.config,
    });

    setAddCommand(window.btoa(conf));
  });

  return (
    <div className="space-y-6">
      <div className="space-y-2">
        <div className="border p-4 border-accent bg-secondary">
          <pre className="font-mono text-sm">{text}</pre>
        </div>
      </div>
      <form
        className="border border-accent bg-secondary p-4 flex flex-col gap-4"
        onSubmit={form.handleSubmit((data) => {})}
      >
        {Object.entries(nonConstFields).map(([key, value]) => (
          <div key={key} className="flex flex-col gap-2">
            <div>
              <span className="font-mono font-bold">{key} </span>
              <span className="text-xs text-muted-foreground">
                {value.description}
              </span>
            </div>
            {matchInput(key, value, form)}
          </div>
        ))}
      </form>
      <div className="flex flex-col gap-2">
        <h3 className="text-lg font-medium">Add To Manifest</h3>
        <p className="text-sm">
          Run the following command in your project directory to automatically
          insert this plugin and config into your manifest.
        </p>
        <CopyBox
          beforeCopy={() => {
            const ajv = new Ajv({
              formats: {
                uint8: {
                  type: "number",
                  async: false,
                  validate: (x) => x >= 0 && x <= 255,
                },
                double: {
                  type: "number",
                  async: false,
                  validate: (x) => typeof x === "number",
                },
              },
            });
            const valid = ajv.validate(schemaData, form.getValues());
            if (!valid) {
              toast.error("Invalid config", {
                description: `Make sure you match the schema correctly: ${ajv.errorsText(
                  ajv.errors,
                )}`,
                important: true,
              });
              return false;
            }
            return true;
          }}
          className="text-sm"
          command={`litehouse add ${id}${addCommand ? `#${addCommand}` : ""}`}
        />
      </div>
    </div>
  );
};

const matchInput = (key: string, value: object, form: any) => {
  const validators = [
    [
      (value) => value.type === "array" && value.minItems === value.maxItems,
      <InputGroup key={key}>
        {value?.items?.map((item, i) => (
          <InputGroupItem
            // biome-ignore lint/suspicious/noArrayIndexKey: can't really do anything about this
            key={`${key}-${i}`}
            {...form.register(`config.${key}[${i}]`, {
              valueAsNumber: item.type === "integer",
            })}
            {...getType(item)}
          />
        ))}
      </InputGroup>,
    ] as const,
    [
      (value) => value.type === "number",
      <Input
        key={key}
        className="input bg-primary-foreground border rounded-lg"
        type="number"
        {...form.register(`config.${key}`, {
          valueAsNumber: true,
        })}
      />,
    ],
  ];

  const result = validators.find(([validator]) => validator(value));
  return result?.[1];
};

const InputGroup = ({ children }) => {
  return <div className="relative flex flex-row">{children}</div>;
};

const InputGroupItem = ({ type, ...props }) => {
  return (
    <input
      className="bg-primary-foreground flex-1 w-0 p-2 [appearance:textfield] first:rounded-l-lg first:border-l border-l-0 last:rounded-r-lg border text-center"
      type={type}
      {...props}
    />
  );
};

// convert into form props
const getType = (props: object): object => {
  const ret = {};

  switch (props.type) {
    case "string":
      ret.type = "text";
      break;
    case "integer":
      ret.type = "number";
      break;
    case "boolean":
      ret.type = "checkbox";
      break;
  }
  if (props.minimum) {
    ret.min = props.minimum;
  }

  return ret;
};
