"use client";

import { useCallback, useEffect, useRef, useState } from "react";

/**
 * A react hook that wraps indexeddb and allows watching specific queries.
 */
export const useIndexedDb = <T>(
  dbName: string,
  storeName: string,
  keyPath: string | string[],
) => {
  const channel = `${dbName}-${storeName}-${keyPath}`;
  const [db, setDb] = useState<IDBDatabase | null>(null);
  const { generation, increment } = useBroadcastGeneration(channel);

  // init the database
  useEffect(() => {
    if (!db && "window" in global) {
      const dbReq = window.indexedDB.open(dbName, 1);
      dbReq.onerror = (event) => {
        console.error("error opening database", event);
      };

      dbReq.onsuccess = (event) => {
        setDb(dbReq.result);
      };

      dbReq.onupgradeneeded = (event) => {
        // @ts-expect-error
        const db = event.target.result;
        const store = db.createObjectStore(storeName, { keyPath });
      };
    }
  }, [storeName, keyPath, dbName, db]);

  const add = useCallback(
    (record: T) => {
      const val = db
        ?.transaction(storeName, "readwrite")
        .objectStore(storeName)
        .put(record);
      if (!val) return;
      val.onsuccess = () => {
        increment();
      };
    },
    [db, increment, storeName],
  );

  const remove = useCallback(
    (id: string) => {
      const val = db
        ?.transaction(storeName, "readwrite")
        .objectStore(storeName)
        .delete(id);
      if (!val) return;
      val.onsuccess = () => {
        increment();
      };
      val.onerror = (e) => {
        console.error(e);
      };
    },
    [db, increment, storeName],
  );

  const subscribe = useCallback(
    <T>(callback: (db: IDBObjectStore) => IDBRequest<T>): T | undefined => {
      const [value, setValue] = useState<T>();
      const inflightGen = useRef<number>(null); // current generation of inflight request
      const valueGen = useRef<number>(null); // current generation of value

      if (!db) {
        return undefined;
      }

      if (
        valueGen.current === generation ||
        inflightGen.current === generation
      ) {
        return value;
      }

      let store: IDBObjectStore;
      try {
        store = db.transaction(storeName).objectStore(storeName);
      } catch (e) {
        // could just be the db is not ready yet
        return value;
      }

      const req = callback(store);

      inflightGen.current = generation;
      req.onsuccess = (event) => {
        if (valueGen.current && valueGen.current > generation) return; // newer data is here
        valueGen.current = generation;
        // @ts-expect-error
        setValue(event.target.result);
      };

      return value;
    },
    [db, generation, storeName],
  );

  return {
    add: db ? add : undefined,
    remove: db ? remove : undefined,
    subscribe,
  };
};

type ManifestItem = {
  id: string;
  name: string;
  version: string;
  downloads?: number;
};

export const useManifestStore = () => {
  const {
    add: addInner,
    subscribe,
    remove: removeInner,
  } = useIndexedDb<ManifestItem>("manifest", "plugins", "id");

  const cb = useCallback(
    (db: IDBObjectStore): IDBRequest<ManifestItem[]> => db.getAll(),
    [],
  );
  const items = subscribe(cb);
  const add = useCallback(
    (plugin: ManifestItem) => {
      addInner?.(plugin);
    },
    [addInner],
  );
  const remove = useCallback(
    (id: string) => {
      removeInner?.(id);
    },
    [removeInner],
  );

  return { add, items, remove };
};

/**
 * A synchronized generation hook that broadcasts changes to a channel.
 */
const useBroadcastGeneration = (channel: string) => {
  const [generation, setGeneration] = useState(0);
  const bc = useRef(new BroadcastChannel(channel));

  bc.current.onmessage = () => {
    setGeneration(generation + 1);
  };

  const increment = useCallback(() => {
    const next = generation + 1;
    bc.current.postMessage(null);
    setGeneration(next);
  }, [generation]);

  return { generation, increment };
};
