// Provides configuration for table entity tests

import { EMULATOR_ACCOUNT_KEY, EMULATOR_ACCOUNT_NAME } from "../../testutils";

export default class TableEntityTestConfig {
  public static protocol: string = "http";
  public static host: string = process.env.AZURITE_TABLE_HOST || "127.0.0.1";
  public static port: number = Number(process.env.AZURITE_TABLE_PORT || "11002");
  public static metadataDbPath = "__tableTestsStorage__";
  public static enableDebugLog: boolean = true;
  public static debugLogPath: string = "g:/debug.log";
  public static accountName: string = EMULATOR_ACCOUNT_NAME;
  public static sharedKey: string = EMULATOR_ACCOUNT_KEY;
}
