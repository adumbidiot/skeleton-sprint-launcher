#include <steam_api.h>

extern "C" {
	bool SteamAPI_ISteamUGC_GetItemInstallInfo(PublishedFileId_t nPublishedFileID, uint64 *punSizeOnDisk, char *pchFolder, uint32 cchFolderSize, uint32* punTimeStamp){
		return SteamUGC()->GetItemInstallInfo(nPublishedFileID, punSizeOnDisk, pchFolder, cchFolderSize, punTimeStamp);
	}
}
