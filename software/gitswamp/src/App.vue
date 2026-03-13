<script setup lang="ts">
import TitleBar from "@/components/layout/TitleBar.vue";
import AppHeader from "@/components/layout/AppHeader.vue";
import RepositoryTabs from "@/components/repository/RepositoryTabs.vue";
import Sidebar from "@/components/repository/Sidebar.vue";
import CommitGraph from "@/components/commits/CommitGraph.vue";
import CommitDetails from "@/components/commits/CommitDetails.vue";
</script>

<template>
  <div class="h-screen w-screen flex flex-col bg-[#121a21] overflow-hidden">
    <TitleBar />
    <RepositoryTabs />
    <AppHeader />
    <div class="flex-1 flex overflow-hidden">
      <Sidebar />
      <CommitGraph />
      <CommitDetails />
    </div>
  </div>
</template>